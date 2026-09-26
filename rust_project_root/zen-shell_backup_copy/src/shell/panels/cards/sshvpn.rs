use super::super::*;

impl Shell {

    pub(crate) fn draw_sshvpn_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
if self.card_show_title { ui::title(v, x + self.scale.s(14.0), y + self.scale.s(10.0), "SSH / VPN", self.scale.fs(11.5), pal.fg); }
        let lines = self.ssh_vpn_lines.clone();
        if lines.is_empty() {
            ui::text_c(v, x + w / 2.0, y + h / 2.0, "No connections", self.scale.fs(9.5), pal.fg, false);
            return;
        }
        let visible = (((h - self.scale.s(36.0)) / self.scale.s(22.0)).floor() as usize).min(lines.len());
        for i in 0..visible {
            let ry = y + self.scale.s(32.0) + i as f32 * self.scale.s(22.0);
            let glyph = if lines[i].starts_with("VPN") { ICON_GLOBE } else { ICON_SHIELD };
            ui::text(v, x + self.scale.s(14.0), ry, glyph, self.scale.fs(10.0), pal.acc, true);
            ui::text(v, x + self.scale.s(30.0), ry, &lines[i], self.scale.fs(9.5), pal.fg, false);
        }
    }

}
