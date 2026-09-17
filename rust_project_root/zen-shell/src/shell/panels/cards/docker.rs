use super::super::*;

impl Shell {

    pub(crate) fn draw_docker_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
if !self.scene_owns_header && self.card_show_title { ui::title(v, x + self.scale.s(14.0), y + self.scale.s(10.0), "Docker", self.scale.fs(11.5), pal.fg); }
        let containers = self.docker_containers.clone();
        let visible = (((h - self.scale.s(36.0)) / self.scale.s(26.0)).floor() as usize).min(containers.len());
        for i in 0..visible {
            let (ref name, ref status, ref image) = containers[i];
            let ry = y + self.scale.s(32.0) + i as f32 * self.scale.s(26.0);
            let dot_col = if status.contains("Up") { GREEN } else { RED };
            v.push(Cmd::Rect { x: x + self.scale.s(14.0), y: ry + self.scale.s(3.0), w: self.scale.s(6.0), h: self.scale.s(6.0), r: 3.0, color: dot_col });
            ui::text(v, x + self.scale.s(24.0), ry, name, self.scale.fs(9.0), pal.fg, false);
            let img_label: String = image.chars().take(20).collect();
            ui::text_r(v, x + w - self.scale.s(14.0), ry, &img_label, self.scale.fs(7.5), pal.fg, false);
        }
        if containers.is_empty() {
            ui::text_c(v, x + w / 2.0, y + h / 2.0, "No containers", self.scale.fs(9.5), pal.fg, false);
        }
    }

}
