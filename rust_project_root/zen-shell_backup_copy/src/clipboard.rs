//! Clipboard manager — `wlr-data-control-unstable-v1` (Hyprland / wlroots).
//!
//! Observes the seat's clipboard selection, keeps a text + image history
//! (persisted to `$XDG_STATE_HOME/zen-shell/clip.json` + `clipimg-*.png`),
//! and re-copies an entry by creating a fresh data source. No external
//! binary, no cliphist daemon — the bar is its own clipboard manager.
//!
//! The panel UI lives in `shell.rs` (`Mode::Clipboard` / `Mode::ClipboardImages`);
//! this module only owns the protocol side + history.

use crate::img::ImageStore;
use crate::shell::Shell;

use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, Proxy};
use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_device_v1::ZwlrDataControlDeviceV1;
use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_manager_v1::ZwlrDataControlManagerV1;
use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_offer_v1::ZwlrDataControlOfferV1;
use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_source_v1::ZwlrDataControlSourceV1;

use std::collections::HashMap;
use std::io::{ErrorKind, Read};
use std::os::fd::{FromRawFd, IntoRawFd};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// How many entries to keep (text + images combined).
const CAP: usize = 64;
/// Text mime types we understand, in preference order.
const TEXT_MIMES: [&str; 2] = ["text/plain;charset=utf-8", "text/plain"];
const IMAGE_MIME: &str = "image/png";

/// State dir base, mirroring `shell::notif_path()`: `$XDG_STATE_HOME/zen-shell`.
pub fn state_dir() -> PathBuf {
    let base = std::env::var("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".local/state")))
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    base.join("zen-shell")
}

pub struct Clipboard {
    pub manager: Option<ZwlrDataControlManagerV1>,
    pub device: Option<ZwlrDataControlDeviceV1>,
    /// text entries, most recent first
    pub texts: Vec<String>,
    /// image entries, most recent first: (img-store key, raw png bytes)
    pub images: Vec<(String, Vec<u8>)>,
    /// offer awaiting its mime list (from the `selection` event)
    pending_offer: Option<ZwlrDataControlOfferV1>,
    /// source proxy id → (mime, bytes) — data for `send` events
    sources: HashMap<u32, (String, Vec<u8>)>,
    next_img: u64,
    /// When we put OUR OWN history back on the clipboard the compositor
    /// echoes that offer back to this manager; reading it would deadlock the
    /// event loop (we'd block in `receive` waiting for a `send` event that
    /// can only be serviced once we return to the loop) → skip that offer.
    skip_own: bool,
}

impl Clipboard {
    /// Construct the manager; history is NOT loaded here. Reading the
    /// persisted history + the user's cliphist bbolt dbs (decoding/rasterizing
    /// every image thumbnail) is real disk+CPU work that would delay the first
    /// frame, so it's deferred to `seed_history()` — called once, right after
    /// the first frame is committed (same pattern as `App::seed_boot_data`).
    pub fn new() -> Self {
        Clipboard {
            manager: None,
            device: None,
            texts: Vec::new(),
            images: Vec::new(),
            pending_offer: None,
            sources: HashMap::new(),
            next_img: 0,
            skip_own: false,
        }
    }

    /// Load persisted history + seed from cliphist's dbs. Runs after the first
    /// frame so the bar appears instantly. Returns true if anything was loaded.
    pub fn seed_history(&mut self, shell: &mut Shell, img: &mut ImageStore) -> bool {
        let before = self.texts.len() + self.images.len();
        self.load_persisted(img);
        self.load_cliphist(shell, img);
        self.texts.len() + self.images.len() > before
    }

    /// Seed history from cliphist's bbolt dbs (if present) so the panels
    /// show the user's long-running cliphist history, not just what
    /// zen-shell has recorded since it started. Live wlr-data-control
    /// recording still prepends new copies on top.
    fn load_cliphist(&mut self, shell: &mut Shell, img: &mut ImageStore) {
        // text db — text entries; images that land in it go to the image list
        if let Some(entries) = crate::cliphist::read_db(&crate::cliphist::text_db_path()) {
            for e in entries {
                if is_image_bytes(&e.data) {
                    self.push_image(e.data, shell, img);
                } else {
                    self.push_text(String::from_utf8_lossy(&e.data).into_owned(), shell);
                }
            }
        }
        // separate image db (the `cliphist -db-path …` setup)
        if let Some(entries) = crate::cliphist::read_db(&crate::cliphist::image_db_path()) {
            for e in entries {
                self.push_image(e.data, shell, img);
            }
        }
    }

    // ------------------------------------------------------------ protocol

    /// Bind the manager global; called once from `App::new`.
    pub fn bind_manager(&mut self, qh: &wayland_client::QueueHandle<crate::App>, globals: &wayland_client::globals::GlobalList) {
        match globals.bind(qh, 1..=2, ()) {
            Ok(m) => self.manager = Some(m),
            Err(e) => {
                if std::env::var("ZEN_TRACE").is_ok() {
                    eprintln!("zen: wlr-data-control unavailable: {e}");
                }
            }
        }
    }

    /// Bind a per-seat device; called when a seat gains keyboard capability.
    pub fn bind_device(&mut self, qh: &wayland_client::QueueHandle<crate::App>, seat: &WlSeat) {
        if self.device.is_some() {
            return;
        }
        let Some(mgr) = self.manager.as_ref() else { return };
        self.device = Some(mgr.get_data_device(seat, qh, ()));
    }

    /// A new offer arrived (data_offer event) — remember it; its mime list
    /// follows as `offer` events, and `selection` confirms it.
    pub fn on_data_offer(&mut self, offer: ZwlrDataControlOfferV1) {
        if let Some(old) = self.pending_offer.take() {
            old.destroy();
        }
        self.pending_offer = Some(offer);
    }

    /// Selection changed. With an offer → read the interesting mime; with
    /// None → the clipboard was cleared (nothing to record).
    pub fn on_selection(&mut self, offer: Option<ZwlrDataControlOfferV1>) {
        if offer.is_some() {
            return; // mime events follow; we read on the first interesting one
        }
        if let Some(o) = self.pending_offer.take() {
            o.destroy();
        }
    }

    /// One mime type of the pending offer. We read the first text/plain (or
    /// image/png) mime we see — the compositor lists all mimes first, so this
    /// is race-free and reads the data once.
    pub fn on_offer_mime(&mut self, conn: &Connection, mime: String, shell: &mut Shell, img: &mut ImageStore) {
        // our own echo offer (we just put history back on the clipboard) — do
        // not read it, or the read blocks the event loop until the 3 s timeout
        if self.skip_own {
            self.skip_own = false;
            self.pending_offer.take().map(|o| o.destroy());
            return;
        }
        let Some(offer) = self.pending_offer.clone() else { return };
        let m = mime.as_str();
        if TEXT_MIMES.contains(&m) {
            if let Some(data) = Self::receive(&offer, m) {
                let text = String::from_utf8_lossy(&data).into_owned();
                self.push_text(text, shell);
            }
            self.pending_offer.take().map(|o| o.destroy());
        } else if m == IMAGE_MIME {
            if let Some(data) = Self::receive(&offer, m) {
                self.push_image(data, shell, img);
            }
            self.pending_offer.take().map(|o| o.destroy());
        }
        let _ = conn;
    }

    /// Request the offer's data for `mime` and read it (bounded, nonblocking).
    fn receive(offer: &ZwlrDataControlOfferV1, mime: &str) -> Option<Vec<u8>> {
        use std::os::fd::AsFd;
        let (r, w) = nix_pipe()?;
        // the compositor writes into the passed (write) end, then closes it;
        // we read the read end until EOF
        offer.receive(mime.to_string(), w.as_fd());
        drop(w);
        Some(read_until_eof(r))
    }

    // ------------------------------------------------------------- history

    fn push_text(&mut self, text: String, shell: &mut Shell) {
        let text = text.trim_end_matches('\n').trim_end_matches('\r').to_string();
        if text.is_empty() {
            return;
        }
        if self.texts.first() == Some(&text) {
            return; // same as current selection — nothing new
        }
        self.texts.retain(|t| t != &text);
        self.texts.insert(0, text);
        self.texts.truncate(CAP / 2);
        shell.clip_text = self.texts.clone();
        self.save();
    }

    /// Decode + downscale to a 64px thumbnail in the img store; returns the
    /// store key — the same `clipimg-{n}.png` name used for the persisted raw
    /// copy, so image history entries have readable names. Undecodable data →
    /// blank tile.
    fn put_thumbnail(&mut self, png: &[u8], img: &mut ImageStore) -> String {
        let thumb = image::load_from_memory(png)
            .ok()
            .and_then(|im| {
                let t = im.thumbnail(64, 64).to_rgba8();
                let (w, h) = t.dimensions();
                let mut px = t.into_raw();
                premultiply(&mut px);
                Some((w, h, px))
            });
        let n = self.next_img;
        self.next_img += 1;
        let key = format!("clipimg-{n}.png");
        if let Some((w, h, px)) = thumb {
            img.put(&key, w, h, px);
        } else {
            img.put(&key, 0, 0, Vec::new()); // undecodable → blank tile
        }
        key
    }

    fn push_image(&mut self, png: Vec<u8>, shell: &mut Shell, img: &mut ImageStore) {
        let key = self.put_thumbnail(&png, img);
        // persist the raw png so history survives restarts
        let _ = std::fs::create_dir_all(state_dir());
        let _ = std::fs::write(state_dir().join(&key), &png);
        self.images.retain(|(_, b)| b != &png);
        self.images.insert(0, (key, png));
        self.images.truncate(CAP / 2);
        shell.clip_images = self.images.iter().map(|(k, _)| k.clone()).collect();
        self.save();
    }

    /// Copy history entry `idx` back to the clipboard (text or image list).
    pub fn set_entry(&mut self, idx: usize, is_image: bool, qh: &wayland_client::QueueHandle<crate::App>) {
        let (mime, bytes) = if is_image {
            let Some((_, png)) = self.images.get(idx) else { return };
            (IMAGE_MIME.to_string(), png.clone())
        } else {
            let Some(t) = self.texts.get(idx) else { return };
            (TEXT_MIMES[0].to_string(), t.as_bytes().to_vec())
        };
        let (Some(mgr), Some(dev)) = (self.manager.as_ref(), self.device.as_ref()) else { return };
        let source = mgr.create_data_source(qh, ());
        source.offer(mime.clone());
        self.sources
            .insert(source.id().protocol_id(), (mime, bytes));
        // sources get no destroy event — the compositor frees them when the
        // selection is replaced; keep the map bounded
        if self.sources.len() > 32 {
            self.sources.clear();
        }
        dev.set_selection(Some(&source));
        // the compositor will echo this offer back to us — skip reading it
        self.skip_own = true;
    }

    /// Copy an arbitrary text (snippets card) to the clipboard — same
    /// data-source path as `set_entry`.
    pub fn copy_text(&mut self, text: String, qh: &wayland_client::QueueHandle<crate::App>) {
        let (Some(mgr), Some(dev)) = (self.manager.as_ref(), self.device.as_ref()) else { return };
        let source = mgr.create_data_source(qh, ());
        source.offer(TEXT_MIMES[0].to_string());
        self.sources.insert(
            source.id().protocol_id(),
            (TEXT_MIMES[0].to_string(), text.as_bytes().to_vec()),
        );
        if self.sources.len() > 32 {
            self.sources.clear();
        }
        dev.set_selection(Some(&source));
        self.skip_own = true;
    }

    /// A source we created is asked for its data — write it, close the fd.
    pub fn on_source_send(&mut self, source: &ZwlrDataControlSourceV1, mime: &str, fd: std::os::fd::OwnedFd) {
        let Some((m, bytes)) = self.sources.get(&source.id().protocol_id()) else { return };
        if m != mime {
            return;
        }
        let raw = fd.into_raw_fd();
        let mut f = unsafe { std::fs::File::from_raw_fd(raw) };
        let _ = std::io::Write::write_all(&mut f, bytes);
    }

    // ----------------------------------------------------------- persistence

    fn persist_path() -> PathBuf {
        state_dir().join("clip.json")
    }

    fn save(&self) {
        use serde::Serialize;
        #[derive(Serialize)]
        struct Disk<'a> {
            texts: &'a [String],
            images: &'a [String],
        }
        // image file names are derived from index at save time — keep it
        // simple: we don't track file names, so persist texts only and let
        // images re-accumulate. (Images are re-persisted as clipimg-N.png on
        // each push; a restart loses the thumbnails but not the clipboard.)
        let _ = std::fs::create_dir_all(state_dir());
        if let Ok(json) = serde_json::to_string(&Disk { texts: &self.texts, images: &[] }) {
            let _ = std::fs::write(Self::persist_path(), json);
        }
    }

    fn load_persisted(&mut self, _img: &mut ImageStore) {
        let Ok(json) = std::fs::read_to_string(Self::persist_path()) else { return };
        // current format: `{texts:[...]}`; old format: a bare string list
        #[derive(serde::Deserialize)]
        struct Disk {
            texts: Vec<String>,
        }
        let texts = serde_json::from_str::<Disk>(&json)
            .map(|d| d.texts)
            .or_else(|_| serde_json::from_str::<Vec<String>>(&json))
            .unwrap_or_default();
        self.texts = texts;
        self.texts.truncate(CAP / 2);
    }
}

// --------------------------------------------------------------------------
// fd helpers
// --------------------------------------------------------------------------

fn nix_pipe() -> Option<(std::os::fd::OwnedFd, std::os::fd::OwnedFd)> {
    use std::os::fd::OwnedFd;
    let mut fds = [0i32; 2];
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return None;
    }
    Some((unsafe { OwnedFd::from_raw_fd(fds[0]) }, unsafe { OwnedFd::from_raw_fd(fds[1]) }))
}

/// Read a pipe until EOF (the compositor closes after writing), nonblocking
/// with a 3 s deadline so a stuck compositor can't wedge the event loop.
fn read_until_eof(fd: std::os::fd::OwnedFd) -> Vec<u8> {
    let raw = fd.into_raw_fd();
    let mut f = unsafe { std::fs::File::from_raw_fd(raw) };
    unsafe {
        let flags = libc::fcntl(raw, libc::F_GETFL);
        libc::fcntl(raw, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
    let mut buf = Vec::new();
    let mut tmp = [0u8; 16384];
    let start = Instant::now();
    loop {
        match f.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                if start.elapsed() > Duration::from_secs(3) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(_) => break,
        }
    }
    buf
}

/// PNG / JPEG / GIF / WEBP / BMP magic → treat the entry as an image
/// (vanilla cliphist stores images in the same db as text).
fn is_image_bytes(d: &[u8]) -> bool {
    d.starts_with(b"\x89PNG\r\n\x1a\n")
        || d.starts_with(b"\xff\xd8\xff")
        || d.starts_with(b"GIF8")
        || (d.len() > 12 && d.starts_with(b"RIFF") && &d[8..12] == b"WEBP")
        || d.starts_with(b"BM")
}

/// Straight alpha → premultiplied RGBA8.
fn premultiply(px: &mut [u8]) {
    for c in px.chunks_exact_mut(4) {
        let a = c[3] as u32;
        if a == 255 {
            continue;
        }
        c[0] = (c[0] as u32 * a / 255) as u8;
        c[1] = (c[1] as u32 * a / 255) as u8;
        c[2] = (c[2] as u32 * a / 255) as u8;
    }
}
