use super::super::*;

impl Shell {

    /// SENSORS — live accelerometer tilt + optional gyro/compass readout.
    pub(crate) fn draw_sensors_card(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, w: f32, h: f32, pal: &Pal) {
if self.card_show_glyph { ui::text(v, x + self.scale.s(12.0), y + self.scale.s(8.0), ICON_SENSORS, self.scale.fs(12.0), pal.acc, true); }
if !self.scene_owns_header && self.card_show_title { ui::title(v, if self.card_show_glyph { x + self.scale.s(30.0) } else { x + self.scale.s(12.0) }, y + self.scale.s(9.0), "Sensors", self.scale.fs(12.0), pal.fg); }
        let s = &self.sensors;
        let none = !s.has_accel && s.heading.is_none() && s.gyro.is_none();
        if none {
            ui::text_c(v, x + w / 2.0, y + h / 2.0 - self.scale.s(6.0), "no IIO sensors", self.scale.fs(9.0), ui::fg3(&pal), false);
            return;
        }
        let mut ry = y + self.scale.s(32.0);
        ui::text(v, x + self.scale.s(16.0), ry, "tilt", self.scale.fs(9.0), ui::fg2(&pal), false);
        ui::text_r(v, x + w - self.scale.s(16.0), ry, format!("pitch {:.0}°  roll {:.0}°", s.pitch, s.roll), self.scale.fs(9.5), pal.fg, false);
        ry += self.scale.s(20.0);
        if let Some(hdg) = s.heading {
            ui::text(v, x + self.scale.s(16.0), ry, "compass", self.scale.fs(9.0), ui::fg2(&pal), false);
            let arrow = match (0.5 + hdg / 360.0 * 8.0) as usize % 8 {
                0 => ICON_GAUGE_0,
                1 => ICON_GAUGE_1,
                2 => ICON_GAUGE_2,
                3 => ICON_GAUGE_3,
                4 => ICON_GAUGE_4,
                5 => ICON_GAUGE_5,
                6 => ICON_GAUGE_6,
                _ => ICON_GAUGE_7,
            };
            ui::text_r(v, x + w - self.scale.s(16.0), ry, format!("{arrow} {:.0}°", hdg), self.scale.fs(9.5), pal.acc, true);
            ry += self.scale.s(20.0);
        }
        if let Some((gx, gy, gz)) = s.gyro {
            ui::text(v, x + self.scale.s(16.0), ry, "gyro °/s", self.scale.fs(9.0), ui::fg2(&pal), false);
            ui::text_r_mono(v, x + w - self.scale.s(16.0), ry, format!("{gx:3.0} {gy:3.0} {gz:3.0}"), self.scale.fs(9.0), pal.fg);
        }
    }

}
