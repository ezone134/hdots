//! //! Lockscreen session glue: PAM verify worker, lock auth results,
//! session unlock.

use super::*;

pub(super) enum LockAuthMsg {
    /// the password verified — unlock
    Ok,
    /// wrong password (or the PAM stack denied) — show the error
    Denied,
    /// PAM genuinely unavailable — log loudly and unlock rather than trap
    Unavailable(String),
}

/// Result of a polkit password check, delivered back to the event loop.
pub(super) enum PolkitAuthMsg {
    /// the password verified — answer the authority and dismiss the dialog
    Accept,
    /// wrong password — show the error and keep the dialog up
    Deny,
}

pub(super) struct LockSurface {
    pub(super) surface: SessionLockSurface,
    pub(super) renderer: Option<Renderer>,
    /// last size the compositor configured us at
    pub(super) configured: (i32, i32),
    pub(super) dirty: bool,
}

impl App {
        pub(super) fn verify_lock_password(&mut self) {
        use crate::shell::Mode;
        if self.shell.mode != Mode::Lock || self.shell.lock_checking {
            return;
        }
        let pw = std::mem::take(&mut self.shell.lock_pw);
        let user = crate::auth::current_user();
        self.shell.lock_checking = true;
        self.shell.lock_error = None;
        let Some(tx) = self.lock_auth_tx.clone() else {
            // no channel yet (startup race) — verify inline rather than drop it
            self.shell.lock_checking = false;
            match crate::auth::verify_password(&user, &pw) {
                Ok(true) => self.unlock_session(),
                Ok(false) => {
                    self.shell.lock_error = Some("Wrong password".to_string());
                    self.dirty = true;
                    self.maybe_render();
                }
                Err(e) => {
                    eprintln!("zen: password check unavailable ({e}) — unlocking anyway");
                    self.unlock_session();
                }
            }
            return;
        };
        std::thread::spawn(move || {
            let msg = match crate::auth::verify_password(&user, &pw) {
                Ok(true) => LockAuthMsg::Ok,
                Ok(false) => LockAuthMsg::Denied,
                Err(e) => LockAuthMsg::Unavailable(e),
            };
            let _ = tx.send(msg);
        });
        self.dirty = true;
        self.maybe_render(); // the field shows “Verifying…”
    }

        pub(super) fn on_lock_auth(&mut self, msg: LockAuthMsg) {
        use crate::shell::Mode;
        if self.shell.mode != Mode::Lock {
            return; // unlocked (or closed) while the check ran
        }
        self.shell.lock_checking = false;
        match msg {
            LockAuthMsg::Ok => self.unlock_session(),
            LockAuthMsg::Denied => {
                self.shell.lock_error = Some("Wrong password".to_string());
                self.dirty = true;
                self.maybe_render();
            }
            LockAuthMsg::Unavailable(e) => {
                eprintln!("zen: password check unavailable ({e}) — unlocking anyway");
                self.unlock_session();
            }
        }
    }

        pub(super) fn unlock_session(&mut self) {
        use crate::shell::Mode;
        self.shell.lock_checking = false;
        self.shell.lock_error = None;
        self.shell.lock_session = false;
        self.apply_mode(Mode::Collapsed);
        if let Some(l) = self.session_lock_active.take() {
            l.unlock();
        }
    }

        pub(super) fn submit_polkit(&mut self) {
        use crate::shell::Mode;
        if self.shell.mode != Mode::PolkitAuth || self.shell.polkit_checking {
            return;
        }
        let pw = std::mem::take(&mut self.shell.polkit_pw);
        let (cookie, uid) = (self.shell.polkit_cookie.clone(), self.shell.polkit_uid);
        self.shell.polkit_checking = true;
        self.shell.polkit_error = None;
        let Some(cmd_tx) = self.polkit_cmd_tx.clone() else {
            // no agent yet (startup race) — surface it rather than drop the pw
            self.shell.polkit_checking = false;
            self.shell.polkit_pw = pw;
            self.shell.polkit_error = Some("polkit agent unavailable".to_string());
            self.dirty = true;
            self.maybe_render();
            return;
        };
        let Some(res_tx) = self.polkit_auth_tx.clone() else {
            self.shell.polkit_checking = false;
            self.shell.polkit_pw = pw;
            return;
        };
        std::thread::spawn(move || {
            let accepted = crate::polkit::verify_password(&pw);
            // answer the polkit authority (the daemon clears its session)
            let _ = cmd_tx.send(PolkitCmd::Response { cookie, uid, accepted });
            // tell the dialog how it went
            let _ = res_tx.send(if accepted { PolkitAuthMsg::Accept } else { PolkitAuthMsg::Deny });
        });
        self.dirty = true;
        self.maybe_render(); // the field shows “Checking…”
    }

        pub(super) fn on_polkit_result(&mut self, msg: PolkitAuthMsg) {
        use crate::shell::Mode;
        if self.shell.mode != Mode::PolkitAuth {
            return; // the dialog was cancelled / closed while the check ran
        }
        self.shell.polkit_checking = false;
        match msg {
            PolkitAuthMsg::Accept => {
                self.shell.polkit_pw.clear();
                self.shell.polkit_cookie.clear();
                self.shell.polkit_error = None;
                self.apply_mode(Mode::Collapsed);
            }
            PolkitAuthMsg::Deny => {
                self.shell.polkit_pw.clear();
                self.shell.polkit_error = Some("Wrong password".to_string());
                self.dirty = true;
                self.maybe_render();
            }
        }
    }
}
