//! Key translation: a Wayland key event + terminal mode → bytes to write to
//! the PTY (escape sequences for special keys, plain UTF-8 otherwise).

use alacritty_terminal::term::TermMode;
use smithay_client_toolkit::seat::keyboard::{KeyEvent, Keysym};

/// What a key press produced.
#[derive(Default)]
pub struct KeyOutcome {
    /// Bytes to write to the PTY.
    pub bytes: Vec<u8>,
    /// Copy the current selection to the clipboard (Ctrl+Shift+C).
    pub copy: bool,
    /// Paste the clipboard into the PTY (Ctrl+Shift+V).
    pub paste: bool,
}

impl KeyOutcome {
    fn text(bytes: impl Into<Vec<u8>>) -> Self {
        Self { bytes: bytes.into(), ..Default::default() }
    }
    fn none() -> Self {
        Self::default()
    }
}

/// Translate a pressed key into PTY bytes (or a clipboard action).
/// Modifiers come from the seat's `update_modifiers` state (SCTK 0.21 does
/// not attach them to the key event itself).
pub fn translate(
    event: &KeyEvent,
    ctrl: bool,
    alt: bool,
    shift: bool,
    mode: &TermMode,
) -> KeyOutcome {
    let sym = event.keysym;
    let app_cursor = mode.contains(TermMode::APP_CURSOR);

    // App shortcuts.
    if ctrl && shift {
        match sym {
            Keysym::c => return KeyOutcome { copy: true, ..Default::default() },
            Keysym::C => return KeyOutcome { copy: true, ..Default::default() },
            Keysym::v => return KeyOutcome { paste: true, ..Default::default() },
            Keysym::V => return KeyOutcome { paste: true, ..Default::default() },
            _ => {}
        }
    }

    // Control characters (Ctrl+letter and friends).
    if ctrl && !alt {
        if let Some(b) = ctrl_byte(sym.raw()) {
            return KeyOutcome::text([b]);
        }
    }

    // Special / editing keys.
    let seq: Option<Vec<u8>> = match sym {
        Keysym::Return | Keysym::KP_Enter => Some(b"\r".to_vec()),
        Keysym::BackSpace => Some(b"\x7f".to_vec()),
        Keysym::Tab => Some(b"\t".to_vec()),
        Keysym::Escape => Some(b"\x1b".to_vec()),
        Keysym::Insert => Some(b"\x1b[2~".to_vec()),
        Keysym::Delete => Some(b"\x1b[3~".to_vec()),
        Keysym::Page_Up => Some(b"\x1b[5~".to_vec()),
        Keysym::Page_Down => Some(b"\x1b[6~".to_vec()),
        Keysym::Home => Some(if app_cursor { b"\x1bOH".to_vec() } else { b"\x1b[H".to_vec() }),
        Keysym::End => Some(if app_cursor { b"\x1bOF".to_vec() } else { b"\x1b[F".to_vec() }),
        Keysym::Up => Some(if app_cursor { b"\x1bOA".to_vec() } else { b"\x1b[A".to_vec() }),
        Keysym::Down => Some(if app_cursor { b"\x1bOB".to_vec() } else { b"\x1b[B".to_vec() }),
        Keysym::Right => Some(if app_cursor { b"\x1bOC".to_vec() } else { b"\x1b[C".to_vec() }),
        Keysym::Left => Some(if app_cursor { b"\x1bOD".to_vec() } else { b"\x1b[D".to_vec() }),
        _ => function_key(sym).map(|seq| seq.to_vec()),
    };
    if let Some(seq) = seq {
        return if alt {
            let mut out = vec![0x1b];
            out.extend_from_slice(&seq);
            KeyOutcome::text(out)
        } else {
            KeyOutcome::text(seq)
        };
    }

    // Alt prefix for printable text.
    if let Some(text) = &event.utf8 {
        if !text.is_empty() && !text.chars().any(|c| c.is_control()) {
            let mut out = Vec::new();
            if alt {
                out.push(0x1b);
            }
            out.extend_from_slice(text.as_bytes());
            return KeyOutcome::text(out);
        }
    }

    KeyOutcome::none()
}

/// Ctrl+<key> → control byte (0x00..0x1f, plus 0x7f for Ctrl+?).
fn ctrl_byte(raw: u32) -> Option<u8> {
    // ASCII letters (xkeysym raw values match ASCII).
    if (0x61..=0x7a).contains(&raw) {
        return Some((raw as u8) & 0x1f);
    }
    if (0x41..=0x5a).contains(&raw) {
        return Some((raw as u8) & 0x1f);
    }
    match raw {
        0x20 => Some(0x00),          // Ctrl+Space → NUL
        0x5b => Some(0x1b),          // Ctrl+[ → ESC
        0x5c => Some(0x1c),          // Ctrl+\ → FS
        0x5d => Some(0x1d),          // Ctrl+] → GS
        0x5e => Some(0x1e),          // Ctrl+^ → RS
        0x5f => Some(0x1f),          // Ctrl+_ → US
        0x3f => Some(0x7f),          // Ctrl+? → DEL
        _ => None,
    }
}

fn function_key(sym: Keysym) -> Option<&'static [u8]> {
    match sym {
        Keysym::F1 => Some(b"\x1bOP"),
        Keysym::F2 => Some(b"\x1bOQ"),
        Keysym::F3 => Some(b"\x1bOR"),
        Keysym::F4 => Some(b"\x1bOS"),
        Keysym::F5 => Some(b"\x1b[15~"),
        Keysym::F6 => Some(b"\x1b[17~"),
        Keysym::F7 => Some(b"\x1b[18~"),
        Keysym::F8 => Some(b"\x1b[19~"),
        Keysym::F9 => Some(b"\x1b[20~"),
        Keysym::F10 => Some(b"\x1b[21~"),
        Keysym::F11 => Some(b"\x1b[23~"),
        Keysym::F12 => Some(b"\x1b[24~"),
        _ => None,
    }
}
