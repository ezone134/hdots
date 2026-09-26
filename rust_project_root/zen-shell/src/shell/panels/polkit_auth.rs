//! Polkit authentication dialog — GNOME/MATE-style password prompt.
//!
//! Shows when a privileged action needs auth. The pill morphs into this
//! centered dialog with: icon, message, user, password field, and
//! Authenticate/Cancel buttons.

use super::*;
use crate::ui;

/// Hit keys for the polkit auth dialog
const POLKIT_KEY_AUTH: u32 = 90;    // Authenticate button
const POLKIT_KEY_CANCEL: u32 = 91;  // Cancel button
const POLKIT_KEY_PW: u32 = 92;      // Password input field

impl Shell {
    pub(crate) fn layout_polkit_auth(&mut self, v: &mut Vec<Cmd>, w: f32, h: f32, pal: &Pal) {
        let pad = 24.0;

        // ── icon (shield glyph) ──
        let icon_x = (w - 48.0) / 2.0;
        let icon_y = pad + 8.0;
        ui::text(v, icon_x, icon_y, ICON_SHIELD, 36.0, pal.acc, true); // nf-fa-shield

        // ── action message ──
        let msg_y = icon_y + 56.0;
        let msg = if self.polkit_message.is_empty() {
            "Authentication required".to_string()
        } else {
            self.polkit_message.clone()
        };
        // Truncate to fit the dialog width
        let max_chars = ((w - pad * 2.0) / (12.0 * 0.62)) as usize;
        let msg_display: String = msg.chars().take(max_chars).collect();
        ui::text_c(v, w / 2.0, msg_y, &msg_display, 13.0, pal.fg, false);

        // ── action_id (smaller, muted) ──
        let act_y = msg_y + 20.0;
        if !self.polkit_action_id.is_empty() {
            let act_display: String = self.polkit_action_id.chars().take(max_chars).collect();
            ui::text_c(v, w / 2.0, act_y, &act_display, 10.0, ui::fg3(pal), false);
        }

        // ── user label ──
        let user_y = act_y + 28.0;
        ui::text_c(v, w / 2.0, user_y, &self.username, 14.0, pal.fg, false);

        // ── password field ──
        let pw_y = user_y + 28.0;
        let pw_w = w - pad * 2.0;
        let pw_h = 36.0;
        let pw_x = pad;

        // field background
        let pw_bg = if self.hover_key == POLKIT_KEY_PW {
            ui::hover_hl(pal)
        } else {
            ui::hover(pal)
        };
        v.push(Cmd::Rect { x: pw_x, y: pw_y, w: pw_w, h: pw_h, r: 8.0, color: pw_bg });

        // password dots or placeholder
        if self.polkit_pw.is_empty() {
            ui::text(v, pw_x + 12.0, pw_y + 10.0, "Password", 13.0, ui::fg3(pal), false);
        } else {
            let dots: String = "*".repeat(self.polkit_pw.len());
            ui::text(v, pw_x + 12.0, pw_y + 10.0, &dots, 13.0, pal.fg, false);
        }

        // cursor blink (show a thin line when focused)
        if self.hover_key == POLKIT_KEY_PW {
            let cursor_x = pw_x + 12.0 + self.polkit_pw.len() as f32 * 13.0 * 0.62;
            v.push(Cmd::Rect {
                x: cursor_x,
                y: pw_y + 8.0,
                w: 2.0,
                h: 20.0,
                r: 1.0,
                color: pal.fg,
            });
        }

        self.region(pw_x, pw_y, pw_w, pw_h, POLKIT_KEY_PW);

        // ── error message ──
        let err_y = pw_y + pw_h + 8.0;
        if let Some(ref err) = self.polkit_error {
            ui::text_c(v, w / 2.0, err_y, err, 11.0, RED, false);
        }

        // ── buttons ──
        let btn_y = h - pad - 36.0;
        let btn_w = 120.0;
        let btn_h = 36.0;
        let btn_gap = 16.0;

        // Cancel button (left)
        let cancel_x = (w - btn_w * 2.0 - btn_gap) / 2.0;
        let cancel_bg = if self.hover_key == POLKIT_KEY_CANCEL {
            ui::hover_hl(pal)
        } else {
            ui::hover(pal)
        };
        v.push(Cmd::Rect { x: cancel_x, y: btn_y, w: btn_w, h: btn_h, r: 8.0, color: cancel_bg });
        ui::text_c(v, cancel_x + btn_w / 2.0, btn_y + 10.0, "Cancel", 13.0, pal.fg, false);
        self.region(cancel_x, btn_y, btn_w, btn_h, POLKIT_KEY_CANCEL);

        // Authenticate button (right)
        let auth_x = cancel_x + btn_w + btn_gap;
        let auth_bg = if self.hover_key == POLKIT_KEY_AUTH {
            mix(pal.acc, pal.fg, 0.15)
        } else {
            pal.acc
        };
        v.push(Cmd::Rect { x: auth_x, y: btn_y, w: btn_w, h: btn_h, r: 8.0, color: auth_bg });
        let auth_label = if self.polkit_checking {
            "Verifying…"
        } else {
            "Authenticate"
        };
        ui::text_c(
            v,
            auth_x + btn_w / 2.0,
            btn_y + 10.0,
            auth_label,
            13.0,
            pal.sfg,
            false,
        );
        self.region(auth_x, btn_y, btn_w, btn_h, POLKIT_KEY_AUTH);
    }
}
