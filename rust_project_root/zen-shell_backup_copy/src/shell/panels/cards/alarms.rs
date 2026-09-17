use super::super::*;

impl Shell {

    /// ALARMS — HH:MM reminders: one row per alarm (time · label), the
    /// composer (`HH:MM label`, Enter) adds, hover ✕ deletes. Firing is
    /// driven by the 1 s pomo timer tick (`alarms_due` → the app pushes a
    /// notification popup); each alarm fires once per day.
    pub(crate) fn draw_alarms_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        self.alarms_rect = (x, y, w, h);
        let pad = self.scale.s(12.0);
        let hdr = self.scale.s(24.0);
        let n = self.alarm_list.len();
        let meta = if n > 0 { format!("{n} set") } else { String::new() };
        if self.card_show_title {
            ui::title(v, x + pad, y + self.scale.s(10.0), "Alarms", self.scale.fs(11.5), pal.fg);
        }
        ui::text_r(v, x + w - pad, y + self.scale.s(12.0), &meta, self.scale.fs(9.0), pal.fg, false);

        // composer row
        let row_h = self.scale.s(24.0);
        let cy0 = y + hdr + self.scale.s(2.0);
        let composing = self.alarm_input.is_some();
        let buf = self.alarm_input.clone().unwrap_or_default();
        v.push(Cmd::Rect { x: x + pad, y: cy0, w: w - pad * 2.0, h: row_h - 2.0, r: self.scale.s(6.0), color: if composing { ui::hover_hl(&pal) } else { ui::hover(&pal) } });
        let shown = if buf.is_empty() { "add alarm — 07:30 stand up" } else { &buf };
        ui::text(v, x + pad + self.scale.s(8.0), cy0 + self.scale.s(5.0), shown, self.scale.fs(8.5), if buf.is_empty() { ui::fg3(&pal) } else { pal.fg }, false);
        self.region(x + pad, cy0, w - pad * 2.0, row_h, ALARM_INPUT_KEY);

        // alarm rows (snapshotted so the ✕ region registration can borrow mutably)
        let y0 = cy0 + row_h;
        let max_rows = (((y + h - 2.0) - y0) / row_h).floor() as usize;
        let rows: Vec<(usize, String, String)> = self
            .alarm_list
            .iter()
            .enumerate()
            .map(|(i, (t, l))| (i, t.clone(), l.clone()))
            .collect();
        for (i, time, label) in rows.iter() {
            let i = *i;
            let time = time.as_str();
            let label = label.as_str();
            if i >= max_rows {
                break;
            }
            let ry = y0 + i as f32 * row_h;
            ui::text_mono(v, x + pad, ry + self.scale.s(3.0), time, self.scale.fs(10.0), pal.acc);
            let name: String = label.chars().take(((w - pad * 2.0 - self.scale.s(60.0)) / self.scale.s(5.9)).max(6.0) as usize).collect();
            ui::text(v, x + pad + self.scale.s(50.0), ry + self.scale.s(3.0), &name, self.scale.fs(9.5), pal.fg, false);
            let del_key = ALARM_DEL_BASE + i as u32;
            if self.hover_key == del_key {
                ui::text(v, x + w - pad - self.scale.s(14.0), ry + self.scale.s(4.0), ICON_CLOSE, self.scale.fs(9.0), RED, true);
                self.region(x + w - pad - self.scale.s(20.0), ry, self.scale.s(20.0), row_h, del_key);
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell() -> Shell {
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        Shell::new(cfg)
    }

    fn pal() -> Pal {
        Pal { fg: 0xe8e6e3ff, bg: 0x141414ff, acc: 0x4fc2ffff, sfg: 0x101010ff }
    }

    #[test]
    fn alarm_add_validates_and_sorts() {
        let mut s = shell();
        s.alarm_input = Some("09:15 stand up".into());
        s.alarm_add();
        s.alarm_input = Some("07:00 early".into());
        s.alarm_add();
        assert_eq!(s.alarm_list.len(), 2);
        assert_eq!(s.alarm_list[0].0, "07:00", "sorted by time");
        // invalid times rejected
        s.alarm_input = Some("25:99 nope".into());
        s.alarm_add();
        s.alarm_input = Some("nope".into());
        s.alarm_add();
        assert_eq!(s.alarm_list.len(), 2);
    }

    #[test]
    fn alarms_due_fires_once_per_day() {
        let mut s = shell();
        let now = unsafe { libc::time(std::ptr::null_mut()) };
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        unsafe { libc::localtime_r(&now, &mut tm) };
        let mut buf = [0u8; 8];
        unsafe {
            libc::strftime(buf.as_mut_ptr() as *mut libc::c_char, buf.len(), c"%H:%M".as_ptr(), &tm);
        }
        let hhmm = String::from_utf8_lossy(&buf[..5]).into_owned();
        s.alarm_list.push((hhmm.clone(), "now!".into()));
        let fire = s.alarms_due();
        assert!(fire.is_some(), "alarm at the current minute fires");
        assert_eq!(fire.unwrap().0, "now!");
        assert!(s.alarms_due().is_none(), "second check the same minute does NOT re-fire");
    }
}
