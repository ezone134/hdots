//! Terminal emulation: alacritty_terminal 0.26 (`Term` + `tty` + `event_loop`).
//!
//! A dedicated thread owns the PTY and the escape-sequence parser; it wakes
//! the UI through a calloop channel. The UI never blocks: it locks the shared
//! `Term` only to render/resize/scroll, which the parser thread does too, so
//! both sides are short critical sections.

use std::borrow::Cow;
use std::sync::Arc;

use alacritty_terminal::event::{Event, EventListener, WindowSize};
use alacritty_terminal::event_loop::{EventLoop, EventLoopSender, Msg, State};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::{Term, TermMode};
use alacritty_terminal::tty::{self, Pty};
use alacritty_terminal::vte::ansi::Rgb;
use calloop::channel::Sender;

use crate::config::Config;

/// Messages from the terminal thread to the UI thread.
pub enum UiMsg {
    /// New content available → redraw.
    Redraw,
    /// Window title change (None = reset to default).
    Title(Option<String>),
    /// Copy text to the clipboard (OSC52 or app shortcut).
    Copy(String),
    /// Read the clipboard and write the formatted result to the PTY.
    Paste(Arc<dyn Fn(&str) -> String + Send + Sync + 'static>),
    /// Reply to a color query (OSC 4/10/11/12 → PTY response).
    ColorRequest(usize, Arc<dyn Fn(Rgb) -> String + Send + Sync + 'static>),
    /// Reply to a text-area-size query.
    TextAreaSize(Arc<dyn Fn(WindowSize) -> String + Send + Sync + 'static>),
    /// The terminal wants to write generated output to the PTY (DA/DSR replies).
    PtyWrite(String),
    /// Bell.
    Bell,
    /// The child exited or the terminal requested shutdown → close the window.
    Exit,
}

/// Our `EventListener`: forwards alacritty terminal events to the UI channel.
#[derive(Clone)]
pub struct TermListener {
    tx: Sender<UiMsg>,
}

impl EventListener for TermListener {
    fn send_event(&self, event: Event) {
        let msg = match event {
            Event::Wakeup | Event::CursorBlinkingChange | Event::MouseCursorDirty => UiMsg::Redraw,
            Event::Title(t) => UiMsg::Title(Some(t)),
            Event::ResetTitle => UiMsg::Title(None),
            Event::ClipboardStore(_, text) => UiMsg::Copy(text),
            Event::ClipboardLoad(_, formatter) => UiMsg::Paste(formatter),
            Event::ColorRequest(index, formatter) => UiMsg::ColorRequest(index, formatter),
            Event::TextAreaSizeRequest(formatter) => UiMsg::TextAreaSize(formatter),
            Event::PtyWrite(text) => UiMsg::PtyWrite(text),
            Event::Bell => UiMsg::Bell,
            Event::Exit | Event::ChildExit(_) => UiMsg::Exit,
        };
        let _ = self.tx.send(msg);
    }
}

/// Terminal grid dimensions (we own this; `Term` reads it via `Dimensions`).
pub struct TermSize {
    pub cols: usize,
    pub rows: usize,
    pub history: usize,
}

impl Dimensions for TermSize {
    fn total_lines(&self) -> usize {
        self.rows + self.history
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

pub struct Terminal {
    pub term: Arc<FairMutex<Term<TermListener>>>,
    sender: EventLoopSender,
    join: Option<std::thread::JoinHandle<(EventLoop<Pty, TermListener>, State)>>,
    history: usize,
}

impl Terminal {
    /// Spawn the shell + the PTY I/O thread. `cols`/`rows` seed the grid; the
    /// real size arrives with the first window configure.
    pub fn spawn(
        cfg: &Config,
        ui_tx: Sender<UiMsg>,
        cols: usize,
        rows: usize,
        cell: (f32, f32),
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let listener = TermListener { tx: ui_tx };
        let size = TermSize { cols, rows, history: cfg.scrollback_lines };
        let mut term = Term::new(cfg.term_config(), &size, listener.clone());
        seed_theme_colors(&mut term, cfg);

        let term = Arc::new(FairMutex::new(term));
        let tty_options = cfg.tty_options();
        let winsize = WindowSize {
            num_lines: rows as u16,
            num_cols: cols as u16,
            cell_width: cell.0 as u16,
            cell_height: cell.1 as u16,
        };
        let pty = tty::new(&tty_options, winsize, 0)?;
        let event_loop = EventLoop::new(term.clone(), listener.clone(), pty, tty_options.drain_on_exit, false)?;
        let sender = event_loop.channel();
        let join = event_loop.spawn();
        Ok(Self { term, sender, join: Some(join), history: cfg.scrollback_lines })
    }

    /// Resize the grid + PTY to the given cell grid.
    pub fn resize(&self, cols: usize, rows: usize, cell: (f32, f32)) {
        {
            let term = self.term.lock();
            if term.columns() != cols || term.screen_lines() != rows {
                drop(term);
                self.term.lock().resize(TermSize { cols, rows, history: self.history });
            }
        }
        let _ = self.sender.send(Msg::Resize(WindowSize {
            num_lines: rows as u16,
            num_cols: cols as u16,
            cell_width: cell.0 as u16,
            cell_height: cell.1 as u16,
        }));
    }

    /// Write raw bytes into the PTY.
    pub fn write(&self, bytes: &[u8]) {
        let _ = self.sender.send(Msg::Input(Cow::Owned(bytes.to_vec())));
    }

    /// Paste text into the PTY, honoring bracketed-paste mode.
    pub fn paste(&self, text: &str) {
        let bracketed = self.term.lock().mode().contains(TermMode::BRACKETED_PASTE);
        if bracketed {
            let out = format!("\x1b[200~{text}\x1b[201~");
            self.write(out.as_bytes());
        } else {
            self.write(text.as_bytes());
        }
    }

    /// Scroll the viewport by `delta` lines (positive = up into history).
    pub fn scroll(&self, delta: i32) {
        self.term.lock().scroll_display(Scroll::Delta(delta));
    }

    /// Set focus state (drives cursor display).
    pub fn set_focused(&self, focused: bool) {
        self.term.lock().is_focused = focused;
    }

    /// Apply a reloaded config to the emulation layer (scrollback + theme).
    pub fn reload_config(&self, cfg: &Config) {
        let mut term = self.term.lock();
        term.set_options(cfg.term_config());
        seed_theme_colors(&mut term, cfg);
    }

    /// Shut down: ask the PTY thread to exit and reap it.
    pub fn shutdown(&mut self) {
        let _ = self.sender.send(Msg::Shutdown);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Seed the terminal's theme table (fg/bg/cursor + 16 ANSI colors) the same
/// way the terminal itself would apply an OSC sequence — this is the only
/// supported way to reach `Term`'s private `colors` field.
fn seed_theme_colors(term: &mut Term<TermListener>, cfg: &Config) {
    let mut s = String::new();
    s.push_str(&format!("\x1b]10;{}\x07", cfg.colors.foreground));
    s.push_str(&format!("\x1b]11;{}\x07", cfg.colors.background));
    s.push_str(&format!("\x1b]12;{}\x07", cfg.colors.cursor));
    for (i, c) in cfg.colors.normal.iter().enumerate() {
        s.push_str(&format!("\x1b]4;{i};{c}\x07"));
    }
    for (i, c) in cfg.colors.bright.iter().enumerate() {
        s.push_str(&format!("\x1b]4;{};{c}\x07", i + 8));
    }
    let mut processor: alacritty_terminal::vte::ansi::Processor<
        alacritty_terminal::vte::ansi::StdSyncHandler,
    > = alacritty_terminal::vte::ansi::Processor::new();
    processor.advance(term, s.as_bytes());
}
