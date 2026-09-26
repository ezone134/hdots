use super::super::*;

impl Shell {

    /// CLOCK & DATE — nothing but the time, the seconds, and the date. No
    /// actions, no buttons — a clean divider-friendly card.
    pub(crate) fn draw_clock_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
        let mut tbuf = [0u8; 16];
        let mut dtbuf = [0u8; 32];
        unsafe {
            let now = libc::time(std::ptr::null_mut());
            let mut tm: libc::tm = std::mem::zeroed();
            libc::localtime_r(&now, &mut tm);
            let n = libc::strftime(tbuf.as_mut_ptr() as *mut libc::c_char, tbuf.len(), c"%H:%M".as_ptr(), &tm);
            let time_s = String::from_utf8_lossy(&tbuf[..n]).into_owned();
            let m = libc::strftime(dtbuf.as_mut_ptr() as *mut libc::c_char, dtbuf.len(), c"%A, %e %b".as_ptr(), &tm);
            let date_s = String::from_utf8_lossy(&dtbuf[..m]).into_owned();
            // big time, centered slightly above the middle
            let size = (h * 0.42).clamp(18.0, 44.0);
            let tw = time_s.chars().count() as f32 * size * 0.62;
            ui::text_hero(v, x + (w - tw) / 2.0, y + (h - size * 2.2) / 2.0, time_s, size, pal.fg);
            ui::text_c(v, x + w / 2.0, y + h - self.scale.s(26.0), date_s, self.scale.fs(10.0), ui::fg2(&pal), false);
        }
    }

}
