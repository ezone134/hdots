use super::*;

impl Shell {
    /// and the connectivity popover rows).
    pub(crate) fn banner_wifi_text(&self) -> String {
        if !self.wifi_on {
            "off".to_string()
        } else if self.ssid.is_empty() {
            "on".to_string()
        } else {
            self.ssid.chars().take(16).collect()
        }
    }

    /// Text of the banner Bluetooth chip (device count when something is
    /// connected).
    pub(crate) fn banner_bt_text(&self) -> String {
        if !self.bt_on {
            "off".to_string()
        } else {
            let n = self.bt_devices.iter().filter(|(_, connected, _)| *connected).count();
            if n > 0 { format!("on \u{00b7} {n}") } else { "on".to_string() }
        }
    }

    /// Summary text of the aggregate connectivity chip: the connected SSID,
    /// or a short radio summary when nothing is on the air.
    pub(crate) fn banner_conn_text(&self) -> String {
        if self.wifi_on && !self.ssid.is_empty() {
            self.ssid.chars().take(14).collect()
        } else if self.wifi_on || self.bt_on {
            "online".to_string()
        } else {
            "offline".to_string()
        }
    }

    /// Live battery percent for the battery chip.
    pub(crate) fn banner_battery_text(&self) -> String {
        format!("{}%", self.battery.max(0))
    }

    /// Weather chip text: (glyph, "22°C"). None until the first weather hit.
    pub(crate) fn banner_weather_text(&self) -> Option<(String, String)> {
        self.weather.as_ref().map(|wx| {
            (crate::weather::describe(wx.code).0.to_string(), format!("{:.0}°C", wx.temp_c))
        })
    }

    /// CPU load percent for the cpu chip.
    pub(crate) fn banner_cpu_text(&self) -> String {
        format!("{}%", self.cpu.max(0))
    }

    /// Memory usage percent for the ram chip.
    pub(crate) fn banner_ram_text(&self) -> String {
        format!("{}%", self.mem_pct.max(0))
    }

    /// Live throughput line for the net-speed chip ("↓5.2M ↑0.8M").
    pub(crate) fn banner_net_text(&self) -> String {
        let d = if self.net_down.is_empty() { "0".to_string() } else { self.net_down.clone() };
        let u = if self.net_up.is_empty() { "0".to_string() } else { self.net_up.clone() };
        format!("↓{d} ↑{u}")
    }

    /// Volume percent for the volume chip.
    pub(crate) fn banner_volume_text(&self) -> String {
        format!("{:.0}%", (self.volume * 100.0).round())
    }

    /// Brightness percent for the brightness chip.
    pub(crate) fn banner_brightness_text(&self) -> String {
        format!("{:.0}%", (self.brightness * 100.0).round())
    }

    /// VPN status word for the vpn chip.
    pub(crate) fn banner_vpn_text(&self) -> String {
        if self.ssh_vpn_lines.is_empty() { "vpn off".to_string() } else { "vpn on".to_string() }
    }

    /// Text width estimator for banner chip labels (mirrors the strip
    /// drawing's font size so hit-testing and rendering never disagree).
    pub(crate) fn banner_label_w(&self, s: &str) -> f32 {
        s.chars().count() as f32 * 0.55 * self.scale.fs(11.5) + self.scale.s(12.0)
    }

    /// Width of a banner chip (excl. the ✕ gutter, which overlays the corner).
    pub(crate) fn banner_chip_w(&self, item: BannerItem) -> f32 {
        match item {
            BannerItem::Workspaces => self.banner_label_w(&(self.ws_active + 1).to_string()),
            BannerItem::Title => {
                let s: String = if self.title.is_empty() {
                    "Desktop".to_string()
                } else {
                    self.title.chars().take(30).collect()
                };
                self.banner_label_w(&s)
            }
            BannerItem::Date => self.banner_label_w(&self.date_str("%a %d")),
            BannerItem::Clock => self.banner_label_w(&self.clock),
            BannerItem::Tray => (self.tray.len().min(8).max(1) as f32) * self.scale.s(22.0) + self.scale.s(14.0),
            BannerItem::Wifi | BannerItem::Bluetooth | BannerItem::Connectivity => {
                let t = match item {
                    BannerItem::Wifi => self.banner_wifi_text(),
                    BannerItem::Bluetooth => self.banner_bt_text(),
                    _ => self.banner_conn_text(),
                };
                self.banner_label_w(&t) + self.scale.s(19.0)
            }
            BannerItem::Battery => self.banner_label_w(&self.banner_battery_text()) + self.scale.s(19.0),
            BannerItem::Weather => {
                let t = self.banner_weather_text().map(|(_, s)| s).unwrap_or_else(|| "--°C".to_string());
                self.banner_label_w(&t) + self.scale.s(19.0)
            }
            BannerItem::Cpu => self.banner_label_w(&self.banner_cpu_text()) + self.scale.s(19.0),
            BannerItem::Ram => self.banner_label_w(&self.banner_ram_text()) + self.scale.s(19.0),
            BannerItem::NetSpeed => self.banner_label_w(&self.banner_net_text()) + self.scale.s(16.0),
            BannerItem::Dnd => self.scale.s(38.0),
            BannerItem::Volume => self.banner_label_w(&self.banner_volume_text()) + self.scale.s(19.0),
            BannerItem::Brightness => self.banner_label_w(&self.banner_brightness_text()) + self.scale.s(19.0),
            BannerItem::Vpn => self.banner_label_w(&self.banner_vpn_text()) + self.scale.s(19.0),
            BannerItem::Settings | BannerItem::Power | BannerItem::AppSearch => self.scale.s(38.0),
            BannerItem::DashToggle => self.banner_label_w(if self.dash_enabled { "Dash: ON" } else { "Dash: OFF" }),
            BannerItem::Branding => {
                let chars = self.brand_glyph.chars().count().max(1) as f32;
                self.scale.s(10.0) + chars * self.scale.fs(13.0) * 0.62 + self.scale.s(20.0)
            }
        }
    }

    /// Fixed width of a banner strip slot (smart fillers are flexible: 0).
    pub(crate) fn banner_token_w(&self, t: &BannerToken) -> f32 {
        match t {
            BannerToken::Chip(c) => self.banner_chip_w(*c),
            BannerToken::Sep(g) => {
                (g.chars().count() as f32 * 0.5 * self.scale.fs(10.5) + self.scale.s(14.0))
                    .max(self.scale.s(14.0))
            }
            BannerToken::Filler => 0.0,
            BannerToken::Zone(_) => 0.0,
        }
    }

    /// Gap between consecutive banner strip slots (px, unscaled).
    pub(crate) fn banner_gap(&self) -> f32 {
        6.0
    }

    /// Pixel rect of banner strip slot `i` (index into `banner_order`) for a
    /// strip spanning `row_w` px. None for slots that aren't rendered right
    /// now (`DashToggle` while not editing). Shared by the drawer, hover and
    /// edit input so they can never disagree.
    pub(crate) fn banner_cell_px(&self, i: usize, row_w: f32) -> Option<(f32, f32, f32, f32)> {
        if let Some(Some((x, w))) = self.banner_cells(row_w).get(i) {
            let y = self.scale.s(BASE_BANNER_Y);
            let h = self.scale.s(BASE_BANNER_H);
            Some((*x, y, *w, h))
        } else {
            None
        }
    }

    /// Lay out the visible strip slots across `row_w`. L/C/R section markers
    /// (`Zone`) split the row into three aligned zones — every token after a
    /// marker until the next one belongs to that zone. LEFT hugs the left
    /// margin of its third, RIGHT hugs the right edge (an item dropped there
    /// "falls" right), CENTER centers within its third. Smart fillers stretch
    /// inside their zone (several split the leftover). With NO zone markers
    /// the tokens flow across the whole row (legacy behavior). One entry per
    /// `banner_order` index — None for slots not rendered right now.
    pub(crate) fn banner_cells(&self, row_w: f32) -> Vec<Option<(f32, f32)>> {
        if !self.banner_chips_enabled {
            // banner chips toggled off — nothing renders, nothing is hit
            return vec![None; self.banner_order.len()];
        }
        let margin = self.scale.s(10.0);
        let gap = self.banner_gap();
        let mut cells: Vec<Option<(f32, f32)>> = vec![None; self.banner_order.len()];
        let mut runs: [Vec<usize>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        let mut zone = 0usize;
        let mut has_zone = false;
        for (i, t) in self.banner_order.iter().enumerate() {
            if matches!(t, BannerToken::Chip(BannerItem::DashToggle)) && !self.dash_edit {
                continue;
            }
            if let Some(z) = t.as_zone() {
                zone = (z as usize).min(2);
                has_zone = true;
                continue; // markers draw nothing
            }
            runs[zone].push(i);
        }
        let avail = (row_w - margin * 2.0).max(0.0);
        // OVERFLOW MODE: the strip's natural width exceeds the row (a panel
        // that shrank to fit the dashboard columns). Lay the tokens in ONE
        // left-aligned flow and pan it with `banner_strip_scroll` — chips
        // never overlap, the scrollbar reveals the rest.
        let natural = self.banner_strip_w();
        if natural > row_w {
            let scroll = self.banner_strip_scroll.min((natural - row_w).max(0.0));
            let mut x = margin - scroll;
            for (i, t) in self.banner_order.iter().enumerate() {
                if matches!(t, BannerToken::Chip(BannerItem::DashToggle)) && !self.dash_edit {
                    continue;
                }
                if t.as_zone().is_some() {
                    continue; // zone markers draw nothing
                }
                let w = if matches!(t, BannerToken::Filler) {
                    0.0
                } else {
                    self.banner_token_w(&self.banner_order[i])
                };
                cells[i] = Some((x, w));
                x += w + gap;
            }
            return cells;
        }
        // Per-run layout: widths then anchor. align 0 = left, 1 = center,
        // 2 = right within `span_w`. Returns (start offset, filler width,
        // occupied width) — `occ` drives the per-zone scroller.
        let layout = |ids: &Vec<usize>, span_w: f32, align: i32| -> (f32, f32, f32) {
            let mut fixed = 0.0f32;
            let mut n_fill = 0usize;
            for &i in ids {
                if matches!(self.banner_order[i], BannerToken::Filler) {
                    n_fill += 1;
                } else {
                    fixed += self.banner_token_w(&self.banner_order[i]);
                }
            }
            let gaps_tot = (ids.len() - 1) as f32 * gap;
            let leftover = (span_w - fixed - gaps_tot).max(0.0);
            let filler_w = if n_fill > 0 { leftover / n_fill as f32 } else { 0.0 };
            let occ = fixed + gaps_tot + if n_fill > 0 { leftover } else { 0.0 };
            let start = match align {
                1 => (span_w - occ).max(0.0) / 2.0,
                2 => (span_w - occ).max(0.0),
                _ => 0.0,
            };
            (start, filler_w, occ)
        };
        let flow = |cells: &mut Vec<Option<(f32, f32)>>,
                    ids: &Vec<usize>,
                    x0: f32,
                    (start, filler_w): (f32, f32),
                    scroll: f32| {
            let mut x = x0 + start - scroll;
            for &i in ids {
                let w = if matches!(self.banner_order[i], BannerToken::Filler) {
                    filler_w
                } else {
                    self.banner_token_w(&self.banner_order[i])
                };
                cells[i] = Some((x, w));
                x += w + gap;
            }
        };
        if has_zone {
            let zw = avail / 3.0;
            let occ_all = self.banner_zone_occ();
            for (z, ids) in runs.iter().enumerate() {
                if ids.is_empty() {
                    continue;
                }
                let lay = layout(ids, zw, z as i32);
                let overflow = (occ_all[z] - zw).max(0.0);
                let off = self.banner_zone_scroll[z].min(overflow);
                flow(&mut cells, ids, margin + z as f32 * zw, (lay.0, lay.1), off);
            }
        } else if !runs[0].is_empty() {
            let lay = layout(&runs[0], avail, 0);
            flow(&mut cells, &runs[0], margin, (lay.0, lay.1), 0.0);
        }
        cells
    }

    /// Occupied width per banner zone (fixed token widths + inter-slot gaps,
    /// fillers / zone markers contribute nothing). The per-zone scroller uses
    /// this to bound the scroll offset: overflow = occ − (zone width / 3).
    pub(crate) fn banner_zone_occ(&self) -> [f32; 3] {
        let gap = self.banner_gap();
        let mut occ = [0.0f32; 3];
        let mut n = [0usize; 3];
        let mut zone = 0usize;
        for t in &self.banner_order {
            if matches!(t, BannerToken::Chip(BannerItem::DashToggle)) && !self.dash_edit {
                continue;
            }
            if let Some(z) = t.as_zone() {
                zone = (z as usize).min(2);
                continue;
            }
            if !matches!(t, BannerToken::Filler) {
                occ[zone] += self.banner_token_w(t);
            }
            n[zone] += 1;
        }
        for z in 0..3 {
            if n[z] > 1 {
                occ[z] += (n[z] - 1) as f32 * gap;
            }
        }
        occ
    }

    /// Scroll banner zone `z` by `steps` wheel notches (positive = reveal
    /// items further right). Returns false when the zone has no overflow or
    /// the offset didn't move (already pinned to an edge) — the caller can
    /// then let the wheel fall through to another target.
    pub(crate) fn banner_zone_scroll_by(&mut self, z: usize, steps: i32) -> bool {
        if steps == 0 || z >= 3 {
            return false;
        }
        let row_w = self.strip_row_w.max(1.0);
        let margin = self.scale.s(10.0);
        let avail = (row_w - margin * 2.0).max(0.0);
        let overflow = (self.banner_zone_occ()[z] - avail / 3.0).max(0.0);
        if overflow <= 0.0 {
            self.banner_zone_scroll[z] = 0.0;
            return false;
        }
        let step = self.scale.s(34.0) * steps as f32;
        let nv = (self.banner_zone_scroll[z] + step).clamp(0.0, overflow);
        if (nv - self.banner_zone_scroll[z]).abs() < 0.5 {
            return false;
        }
        self.banner_zone_scroll[z] = nv;
        true
    }

    /// Zone under `(px, py)` when the pointer is over the banner strip band
    /// and the strip is zoned (L / C / R). None otherwise — lets wheel input
    /// scroll exactly one zone instead of the whole board.
    pub(crate) fn banner_zone_at(&self, px: f32, py: f32) -> Option<usize> {
        if self.mode != Mode::Expanded || self.banner_collapsed() {
            return None;
        }
        if !self.banner_order.iter().any(|t| t.as_zone().is_some()) {
            return None;
        }
        let top = self.banner_strip_top();
        let h = self.banner_strip_h();
        if py < top || py > top + h {
            return None;
        }
        let row_w = self.strip_row_w.max(1.0);
        let margin = self.scale.s(10.0);
        let avail = (row_w - margin * 2.0).max(0.0);
        if avail <= 0.0 || px < margin || px > margin + avail {
            return None;
        }
        Some((((px - margin) / (avail / 3.0)).floor() as usize).min(2))
    }

    /// True when the WHOLE banner strip overflows the panel width — i.e. the
    /// strip has more chips than fit and becomes a single horizontally-
    /// scrollable row (the no-zone-marker / narrow-panel case).
    pub(crate) fn banner_strip_overflow(&self) -> bool {
        if !self.banner_chips_enabled || self.banner_collapsed() {
            return false;
        }
        self.banner_strip_w() > self.strip_row_w.max(1.0)
    }

    /// True when `(px, py)` hovers the expansion's banner strip band (normal
    /// or edit mode) and the whole-strip scroller owns the wheel.
    pub(crate) fn banner_strip_point(&self, px: f32, py: f32) -> bool {
        if self.mode != Mode::Expanded || self.banner_collapsed() {
            return false;
        }
        let top = self.banner_strip_top();
        let h = self.banner_strip_h();
        if py < top || py > top + h {
            return false;
        }
        let row_w = self.strip_row_w.max(0.0);
        px >= 0.0 && px <= row_w
    }

    /// Publish the viewing strip's visible cells into the scene values under
    /// `banner_cells_v` — the declarative `BannerRow`'s data source. Geometry
    /// comes straight from `banner_cells(w)` (the exact function the Rust
    /// drawer hit-tests with, so the two can never disagree); inks mirror the
    /// drawer's per-chip color ladders. Also seeds the
    /// `dash_chips_via_scene` flag: when cells actually published, the Rust
    /// chip painter suppresses itself so the strip is never drawn twice.
    /// Returns the number of visible cells (0 = nothing rendered, flag stays
    /// down).
    pub(crate) fn publish_banner_cells(&mut self, vals: &mut crate::scene::SceneValues, w: f32) -> usize {
        use crate::scene::{BannerCell, BannerCellKind, SceneValue};
        use crate::shell::panels::dashboard::TEXT_Y_BIAS;
        let _ = TEXT_Y_BIAS; // baselines live in the scene item; width math already matches
        if self.mode != Mode::Expanded || self.banner_collapsed() {
            self.dash_chips_via_scene = false;
            return 0;
        }
        let s = self.scale.s(1.0);
        let _ = s; // (documenting that all insets below are base px)
        let strip_top = self.scale.s(crate::shell::BASE_BANNER_Y);
        let strip_h = self.scale.s(crate::shell::BASE_BANNER_H);
        let _ = (strip_top, strip_h); // the RON band declares the box; cells are relative
        let cells_geo = self.banner_cells(w);
        let mut cells: Vec<BannerCell> = Vec::with_capacity(self.banner_order.len());
        let mut zone = 0u8;
        let mut zones = false;
        for (i, item) in self.banner_order.iter().enumerate() {
            if let Some(z) = item.as_zone() {
                zone = (z as u8).min(2);
                zones = true;
                continue;
            }
            let Some((cx, cw)) = cells_geo[i] else { continue };
            let key = if matches!(item, BannerToken::Filler) { 0 } else { BANNER_KEY_BASE + i as u32 };
            let hov = if self.dash_edit {
                false // edit chrome stays Rust; the scene only renders viewing
            } else {
                self.hover_key == key
            };
            let col = |name: &str| name.to_string();
            let cell = match item {
                BannerToken::Chip(it) => match it {
                    BannerItem::Workspaces => BannerCell {
                        x: cx,
                        w: cw,
                        ink: col(if hov { "bchip_acc" } else { "bchip_workspaces" }),
                        text: (self.ws_active + 1).to_string(),
                        glyph: None,
                        icons: Vec::new(),
                        zone: if zones { zone } else { 3 },
                        kind: BannerCellKind::Text { n: None },
                        key,
                    },
                    BannerItem::Title => BannerCell {
                        x: cx,
                        w: cw,
                        ink: col("bchip_title"),
                        text: if self.title.is_empty() { "Desktop".to_string() } else { self.title.clone() },
                        glyph: None,
                        icons: Vec::new(),
                        zone: if zones { zone } else { 3 },
                        kind: BannerCellKind::Text { n: Some(30) },
                        key,
                    },
                    BannerItem::Date => BannerCell {
                        x: cx,
                        w: cw,
                        ink: col("bchip_date"),
                        text: self.date_str("%a %d"),
                        glyph: None,
                        icons: Vec::new(),
                        zone: if zones { zone } else { 3 },
                        kind: BannerCellKind::Text { n: None },
                        key,
                    },
                    BannerItem::Clock => BannerCell {
                        x: cx,
                        w: cw,
                        ink: col("bchip_clock"),
                        text: self.clock.clone(),
                        glyph: None,
                        icons: Vec::new(),
                        zone: if zones { zone } else { 3 },
                        kind: BannerCellKind::Text { n: None },
                        key,
                    },
                    BannerItem::Tray => BannerCell {
                        x: cx,
                        w: cw,
                        ink: col("bchip_tray"),
                        text: String::new(),
                        glyph: None,
                        icons: (0..self.tray.len().min(8)).map(|ti| self.tray[ti].image_key()).collect(),
                        zone: if zones { zone } else { 3 },
                        kind: BannerCellKind::Tray,
                        key,
                    },
                    BannerItem::Settings => BannerCell {
                        x: cx,
                        w: cw,
                        ink: col(if hov { "bchip_acc" } else { "bchip_settings" }),
                        text: String::new(),
                        glyph: Some(crate::icons::ICON_SETTINGS_FILL.to_string()),
                        icons: Vec::new(),
                        zone: if zones { zone } else { 3 },
                        kind: BannerCellKind::Icon { size: 14.0 },
                        key,
                    },
                    BannerItem::Power => BannerCell {
                        x: cx,
                        w: cw,
                        ink: col(if hov { "bchip_acc" } else { "bchip_power" }),
                        text: String::new(),
                        glyph: Some(crate::icons::ICON_POWER_FILL.to_string()),
                        icons: Vec::new(),
                        zone: if zones { zone } else { 3 },
                        kind: BannerCellKind::Icon { size: 14.0 },
                        key,
                    },
                    BannerItem::AppSearch => BannerCell {
                        x: cx,
                        w: cw,
                        ink: col(if hov { "bchip_acc" } else { "bchip_search" }),
                        text: String::new(),
                        glyph: Some(crate::icons::ICON_SEARCH_FILL.to_string()),
                        icons: Vec::new(),
                        zone: if zones { zone } else { 3 },
                        kind: BannerCellKind::Icon { size: 14.0 },
                        key,
                    },
                    BannerItem::Wifi => BannerCell {
                        x: cx,
                        w: cw,
                        ink: col("bchip_wifi"),
                        text: self.banner_wifi_text(),
                        glyph: Some(crate::icons::ICON_WIFI_FILL.to_string()),
                        icons: Vec::new(),
                        zone: if zones { zone } else { 3 },
                        kind: BannerCellKind::IconLabel,
                        key,
                    },
                    BannerItem::Bluetooth => BannerCell {
                        x: cx,
                        w: cw,
                        ink: col("bchip_bt"),
                        text: self.banner_bt_text(),
                        glyph: Some(crate::icons::ICON_BLUETOOTH_FILL.to_string()),
                        icons: Vec::new(),
                        zone: if zones { zone } else { 3 },
                        kind: BannerCellKind::IconLabel,
                        key,
                    },
                    BannerItem::Connectivity => BannerCell {
                        x: cx,
                        w: cw,
                        ink: col("bchip_conn"),
                        text: self.banner_conn_text(),
                        glyph: Some(crate::icons::ICON_LINK.to_string()),
                        icons: Vec::new(),
                        zone: if zones { zone } else { 3 },
                        kind: BannerCellKind::IconLabel,
                        key,
                    },
                    BannerItem::Battery => BannerCell {
                        x: cx,
                        w: cw,
                        ink: col("bchip_battery"),
                        text: self.banner_battery_text(),
                        glyph: Some(crate::ui::battery_glyph(self.battery, self.ac_online).to_string()),
                        icons: Vec::new(),
                        zone: if zones { zone } else { 3 },
                        kind: BannerCellKind::IconLabel,
                        key,
                    },
                    BannerItem::Weather => {
                        let (glyph, text) = self
                            .banner_weather_text()
                            .map(|(g, t)| (Some(g), t))
                            .unwrap_or((Some(crate::icons::ICON_THERMOSTAT.to_string()), "--°C".to_string()));
                        BannerCell {
                            x: cx,
                            w: cw,
                            ink: col("bchip_weather"),
                            text,
                            glyph,
                            icons: Vec::new(),
                            zone: if zones { zone } else { 3 },
                            kind: BannerCellKind::IconLabel,
                            key,
                        }
                    }
                    BannerItem::Cpu => BannerCell {
                        x: cx,
                        w: cw,
                        ink: col("bchip_cpu"),
                        text: self.banner_cpu_text(),
                        glyph: Some(crate::icons::ICON_SPEED_FILL.to_string()),
                        icons: Vec::new(),
                        zone: if zones { zone } else { 3 },
                        kind: BannerCellKind::IconLabel,
                        key,
                    },
                    BannerItem::Ram => BannerCell {
                        x: cx,
                        w: cw,
                        ink: col("bchip_ram"),
                        text: self.banner_ram_text(),
                        glyph: Some(crate::icons::ICON_MEMORY.to_string()),
                        icons: Vec::new(),
                        zone: if zones { zone } else { 3 },
                        kind: BannerCellKind::IconLabel,
                        key,
                    },
                    BannerItem::NetSpeed => BannerCell {
                        x: cx,
                        w: cw,
                        ink: col("bchip_net"),
                        text: self.banner_net_text(),
                        glyph: None,
                        icons: Vec::new(),
                        zone: if zones { zone } else { 3 },
                        kind: BannerCellKind::TextAcc,
                        key,
                    },
                    BannerItem::Dnd => BannerCell {
                        x: cx,
                        w: cw,
                        ink: col("bchip_dnd"),
                        text: String::new(),
                        glyph: Some(
                            if self.dnd {
                                crate::icons::ICON_BELL_OFF_FILL
                            } else {
                                crate::icons::ICON_BELL_FILL
                            }
                            .to_string(),
                        ),
                        icons: Vec::new(),
                        zone: if zones { zone } else { 3 },
                        kind: BannerCellKind::Icon { size: 14.0 },
                        key,
                    },
                    BannerItem::Volume => BannerCell {
                        x: cx,
                        w: cw,
                        ink: col("bchip_volume"),
                        text: self.banner_volume_text(),
                        glyph: Some(if self.volume_muted { crate::icons::ICON_VOLUME_OFF_FILL } else { crate::icons::ICON_VOLUME_FILL }.to_string()),
                        icons: Vec::new(),
                        zone: if zones { zone } else { 3 },
                        kind: BannerCellKind::IconLabel,
                        key,
                    },
                    BannerItem::Brightness => BannerCell {
                        x: cx,
                        w: cw,
                        ink: col("bchip_bright"),
                        text: self.banner_brightness_text(),
                        glyph: Some(crate::icons::ICON_BRIGHTNESS_FILL.to_string()),
                        icons: Vec::new(),
                        zone: if zones { zone } else { 3 },
                        kind: BannerCellKind::IconLabel,
                        key,
                    },
                    BannerItem::Vpn => BannerCell {
                        x: cx,
                        w: cw,
                        ink: col("bchip_vpn"),
                        text: self.banner_vpn_text(),
                        glyph: Some(crate::icons::ICON_LOCK_FILL.to_string()),
                        icons: Vec::new(),
                        zone: if zones { zone } else { 3 },
                        kind: BannerCellKind::IconLabel,
                        key,
                    },
                    // DashToggle only renders in edit mode, which the scene
                    // never covers — skip it here so widths and keys stay put
                    BannerItem::DashToggle => continue,
                    BannerItem::Branding => BannerCell {
                        x: cx,
                        w: cw,
                        ink: col("bchip_branding"),
                        text: self.brand_glyph.clone(),
                        glyph: None,
                        icons: Vec::new(),
                        zone: if zones { zone } else { 3 },
                        kind: BannerCellKind::Brand,
                        key,
                    },
                },
                BannerToken::Sep(g) => BannerCell {
                    x: cx,
                    w: cw,
                    ink: col("bchip_sep"),
                    text: g.clone(),
                    glyph: None,
                    icons: Vec::new(),
                    zone: if zones { zone } else { 3 },
                    kind: BannerCellKind::Sep,
                    key,
                },
                BannerToken::Filler => BannerCell {
                    x: cx,
                    w: cw,
                    ink: "bchip_idle".into(),
                    text: String::new(),
                    glyph: None,
                    icons: Vec::new(),
                    zone: if zones { zone } else { 3 },
                    kind: BannerCellKind::Filler,
                    key: 0,
                },
                BannerToken::Zone(_) => continue,
            };
            cells.push(cell);
        }
        let n = cells.len();
        vals.insert("banner_cells_v", SceneValue::BannerCells(cells));
        self.dash_chips_via_scene = n > 0;
        n
    }

    /// Scroll the WHOLE banner strip by `steps` wheel notches (positive =
    /// reveal items further right). Returns false when the strip doesn't
    /// overflow or the offset didn't move — the caller then lets the wheel
    /// fall through to the board.
    pub(crate) fn banner_strip_scroll_by(&mut self, steps: i32) -> bool {
        if steps == 0 {
            return false;
        }
        let row_w = self.strip_row_w.max(1.0);
        let overflow = (self.banner_strip_w() - row_w).max(0.0);
        if overflow <= 0.0 {
            self.banner_strip_scroll = 0.0;
            return false;
        }
        let step = self.scale.s(34.0) * steps as f32;
        let nv = (self.banner_strip_scroll + step).clamp(0.0, overflow);
        if (nv - self.banner_strip_scroll).abs() < 0.5 {
            return false;
        }
        self.banner_strip_scroll = nv;
        true
    }

    /// Insertion index for a banner strip drop: `from` = the dragged token's
    /// original index, `k` = the column chosen on the ORIGINAL list (0..=n0),
    /// `n0` = original length, `from_tray` = dragging in from the parked tray.
    /// Returns the index on the post-removal list at which the token must be
    /// inserted (boundaries clamp; removed slots shift left by one).
    #[cfg(test)]
    pub(crate) fn banner_drop_target(from: usize, k: usize, n0: usize, from_tray: bool) -> usize {
        let from = from.min(n0);
        let n1 = if from_tray { n0 } else { n0.saturating_sub(1) };
        if from_tray {
            k.min(n1)
        } else if k <= from {
            k.min(n1)
        } else {
            (k - 1).min(n1)
        }
    }

    /// Natural (un-stretched) width of the rendered banner strip — chips and
    /// separators packed from the left margin, fillers contributing nothing.
    pub(crate) fn banner_strip_w(&self) -> f32 {
        let margin = self.scale.s(10.0);
        if !self.banner_chips_enabled {
            return margin * 2.0;
        }
        let mut fixed = 0.0;
        let mut n = 0usize;
        for t in &self.banner_order {
            if matches!(t, BannerToken::Chip(BannerItem::DashToggle)) && !self.dash_edit {
                continue;
            }
            if t.as_zone().is_some() {
                continue;
            }
            n += 1;
            if !matches!(t, BannerToken::Filler) {
                fixed += self.banner_token_w(t);
            }
        }
        if n == 0 {
            return margin * 2.0;
        }
        margin * 2.0 + fixed + (n - 1) as f32 * self.banner_gap()
    }

    /// Banner items NOT on the strip, in canonical order (the parked tray).
    pub(crate) fn unused_banner_items(&self) -> Vec<BannerItem> {
        DEFAULT_BANNER_ORDER
            .iter()
            .copied()
            .filter(|b| !self.banner_order.iter().any(|t| *t == BannerToken::Chip(*b)))
            .collect()
    }

    /// Every token on the parked "Banner chips" tray (edit mode): the chips the
    /// spec's `banner.parked` declares (in declared order — falling back to the
    /// DEFAULT chip pool), then all separator presets, then the smart filler.
    /// Clicking any of them appends it to the strip; dragging drops it at a
    /// column. Index shared by the drawer and edit input via `banner_tray_chip_px`.
    pub(crate) fn parked_banner_tokens(&self) -> Vec<BannerToken> {
        let mut v: Vec<BannerToken> = if !self.cfg.dashboard.banner.parked.is_empty() {
            self.cfg
                .dashboard
                .banner
                .parked
                .iter()
                .filter_map(|s| BannerToken::parse(s))
                .filter(|t| matches!(t, BannerToken::Chip(_)))
                // chips already on the strip aren't offered again
                .filter(|t| !self.banner_order.iter().any(|st| st == t))
                .collect()
        } else {
            self.unused_banner_items()
                .iter()
                .map(|&b| BannerToken::Chip(b))
                .collect()
        };
        v.extend(BANNER_SEP_PRESETS.iter().map(|g| BannerToken::Sep(g.to_string())));
        // NOTE: the L/C/R zone markers are NOT offered as chips here. They
        // are added automatically when a chip is clicked from the tray
        // (banner_home_zone), so every strip keeps its three physical
        // sections without forcing users to pick "left/center/right".
        v.push(BannerToken::Filler);
        v
    }

    /// Pixel rect of banner parked-tray chip `j` (index into
    /// `unused_banner_items`) — shared by the drawer and edit input. The
    /// parked chips flow left→right inside the banner tray rect and every
    /// column STRETCHES so the row always fills the tray's full width (no
    /// dead space hugging the right edge on wide windows).
    pub(crate) fn banner_tray_chip_px(&self, j: usize) -> (f32, f32, f32, f32) {
        pub(crate) const MIN_CW: f32 = 96.0;
        pub(crate) const CH: f32 = 44.0;
        let (tx, ty, tw, _) = self.banner_tray_rect;
        let per_row = (((tw - 24.0) / (MIN_CW + 12.0)).floor() as usize).max(1);
        let cw = ((tw - 24.0 - (per_row as f32 - 1.0) * 12.0) / per_row as f32).max(MIN_CW);
        let col = j % per_row;
        let row = j / per_row;
        (
            tx + 12.0 + col as f32 * (cw + 12.0),
            ty + 34.0 + (row - self.banner_tray_scroll_row as usize) as f32 * (CH + 10.0),
            cw,
            CH,
        )
    }

    /// Content height of the banner-chips parked tray strip (edit mode).
    /// Mirrors the drawer's chip wrapping so `grid_top` and the drawing
    /// always agree even before the tray rect is set.
    pub(crate) fn banner_tray_h(&self) -> f32 {
        if self.banner_chips_absent() {
            return 0.0;
        }
        let n = self.parked_banner_tokens().len();
        if n == 0 {
            return self.scale.s(44.0);
        }
        let cw_chip = self.scale.s(96.0);
        let ch_chip = self.scale.s(44.0);
        let tray_w = self.dash_canvas_w().min(self.dash_view_w()) - self.scale.s(28.0);
        let per_row = ((tray_w - self.scale.s(24.0)) / (cw_chip + self.scale.s(12.0))).floor().max(1.0) as usize;
        let chip_rows = ((n + per_row - 1) / per_row).max(1);
        let visible_rows = chip_rows.min(BANNER_TRAY_VROWS);
        // 34 px title row — leaves ~12 px of air below the tray's "clear all"
        // pill (which sits at +6 with a 16 px height) before the chip rows.
        self.scale.s(34.0) + visible_rows as f32 * (ch_chip + self.scale.s(10.0)) + self.scale.s(8.0)
    }

    /// The banner-chips tray top (edit mode) — sits directly above the
    /// dashboard cards tray.
    pub(crate) fn banner_tray_top(&self) -> f32 {
        // the tray strip floats just under the banner chips; the card tray
        // then sits below this strip
        self.tray_at_top()
    }

    /// Top Y of the strip-editor row (edit mode, dashboard OFF) — directly
    /// below the edit-mode UI controls row (pad / win / card), which itself
    /// sits directly below the banner-chips parking tray.
    pub(crate) fn banner_ctrl_top(&self) -> f32 {
        self.ui_ctrl_top() + self.ui_ctrl_h() + self.tray_gap()
    }

    /// Height of the strip-editor row (show dashboard / separator presets /
    /// smart filler / banner width buttons).
    pub(crate) fn banner_ctrl_h(&self) -> f32 {
        self.scale.s(36.0)
    }

    /// Minimum strip-editor row width so every button stays reachable — the
    /// banner-only dashboard (dashboard OFF, edit mode) grows to at least
    /// this, mirroring the drawer's text estimates.
    pub(crate) fn banner_ctrl_min_w(&self) -> f32 {
        let fs = self.scale.fs(9.5);
        let tw = |s: &str| s.chars().count() as f32 * 0.62 * fs;
        let gap = self.scale.s(8.0);
        let mut left = self.scale.s(28.0) + tw("Show dashboard cards");
        for g in BANNER_SEP_PRESETS {
            left += gap + self.scale.s(22.0) + tw(g);
        }
        left += gap + self.scale.s(28.0) + tw("spacer");
        let right = self.scale.s(20.0) * 2.0 + gap * 2.0 + tw("width 0000") + self.scale.s(20.0);
        // + the always-visible "Enable banner chips" toggle on the far right
        let mut right = right + gap + self.scale.s(28.0) + tw("Enable banner chips: OFF");
        // + the card-header visibility toggles beside it (dashboard ON only)
        if self.dash_enabled {
            right += gap + self.scale.s(20.0) + tw("Card glyphs: OFF");
            right += gap + self.scale.s(20.0) + tw("Card titles: OFF");
        }
        left + self.scale.s(12.0) + right + self.scale.s(30.0)
    }

    // ── tray pager helpers ──────────────────────────────────────────────

    /// Total number of chip rows inside the banner-chips tray.
    pub(crate) fn banner_tray_rows(&self) -> usize {
        let n = self.parked_banner_tokens().len();
        if n == 0 { return 0; }
        let tw = self.banner_tray_rect.2;
        if tw <= 0.0 { return 0; }
        let cw = self.scale.s(96.0);
        let per_row = (((tw - self.scale.s(24.0)) / (cw + self.scale.s(12.0))).floor() as usize).max(1);
        ((n + per_row - 1) / per_row).max(1)
    }

    /// True when the banner-chips tray has more rows than `BANNER_TRAY_VROWS`.
    pub(crate) fn banner_tray_overflow(&self) -> bool {
        let n = self.parked_banner_tokens().len();
        if n == 0 { return false; }
        let tw = self.banner_tray_rect.2;
        if tw <= 0.0 { return false; }
        let cw = self.scale.s(96.0);
        let per_row = (((tw - self.scale.s(24.0)) / (cw + self.scale.s(12.0))).floor() as usize).max(1);
        let rows = ((n + per_row - 1) / per_row).max(1);
        rows as i32 > BANNER_TRAY_VROWS as i32
    }

    /// Maximum (clamped) scroll-row for the banner-chips tray.
    pub(crate) fn banner_tray_max_scroll(&self) -> i32 {
        let n = self.parked_banner_tokens().len();
        if n == 0 { return 0; }
        let tw = self.banner_tray_rect.2;
        if tw <= 0.0 { return 0; }
        let cw = self.scale.s(96.0);
        let per_row = (((tw - self.scale.s(24.0)) / (cw + self.scale.s(12.0))).floor() as usize).max(1);
        let rows = ((n + per_row - 1) / per_row).max(1);
        (rows as i32 - BANNER_TRAY_VROWS as i32).max(0)
    }

    /// Scroll the banner-chips tray by `steps` rows (+1 / −1 per notch).
    /// Returns true when the offset actually changed (dirty redraw needed).
    pub(crate) fn banner_tray_scroll_by(&mut self, steps: i32) -> bool {
        let max = self.banner_tray_max_scroll();
        let nv = (self.banner_tray_scroll_row + steps).clamp(0, max);
        if nv != self.banner_tray_scroll_row {
            self.banner_tray_scroll_row = nv;
            true
        } else {
            false
        }
    }

    /// True when chip `j` (global index into `parked_banner_tokens`) falls
    /// inside the pager's visible row window.
    pub(crate) fn banner_tray_row_vis(&self, j: usize) -> bool {
        let tw = self.banner_tray_rect.2;
        if tw <= 0.0 { return false; }
        let cw = self.scale.s(96.0);
        let per_row = (((tw - self.scale.s(24.0)) / (cw + self.scale.s(12.0))).floor() as usize).max(1);
        let row = j / per_row;
        let s = self.banner_tray_scroll_row as usize;
        row >= s && row < s + BANNER_TRAY_VROWS
    }

    // ── parked-cards tray pager ─────────────────────────────────────────

    pub(crate) fn dash_tray_overflow(&self) -> bool {
        if self.tray_cards.is_empty() { return false; }
        let tw = self.tray_rect.2;
        if tw <= 0.0 { return false; }
        let cw = self.scale.s(96.0);
        let per_row = (((tw - self.scale.s(24.0)) / (cw + self.scale.s(12.0))).floor() as usize).max(1);
        let rows = ((self.tray_cards.len() + per_row - 1) / per_row).max(1);
        rows as i32 > DASH_TRAY_VROWS as i32
    }

    pub(crate) fn dash_tray_max_scroll(&self) -> i32 {
        if self.tray_cards.is_empty() { return 0; }
        let tw = self.tray_rect.2;
        if tw <= 0.0 { return 0; }
        let cw = self.scale.s(96.0);
        let per_row = (((tw - self.scale.s(24.0)) / (cw + self.scale.s(12.0))).floor() as usize).max(1);
        let rows = ((self.tray_cards.len() + per_row - 1) / per_row).max(1);
        (rows as i32 - DASH_TRAY_VROWS as i32).max(0)
    }

    pub(crate) fn dash_tray_scroll_by(&mut self, steps: i32) -> bool {
        let max = self.dash_tray_max_scroll();
        let nv = (self.dash_tray_scroll_row + steps).clamp(0, max);
        if nv != self.dash_tray_scroll_row {
            self.dash_tray_scroll_row = nv;
            true
        } else {
            false
        }
    }

    pub(crate) fn dash_tray_row_vis(&self, i: usize) -> bool {
        let tw = self.tray_rect.2;
        if tw <= 0.0 { return false; }
        let cw = self.scale.s(96.0);
        let per_row = (((tw - self.scale.s(24.0)) / (cw + self.scale.s(12.0))).floor() as usize).max(1);
        let row = i / per_row;
        let s = self.dash_tray_scroll_row as usize;
        row >= s && row < s + DASH_TRAY_VROWS
    }

    // ── edit-mode UI controls row (below parked-cards tray) ─────────────

    /// Height of the edit-mode UI controls row ("pad / win / card" sliders
    /// + the radius-sync toggle). Doubles when the row wraps to two levels.
    pub(crate) fn ui_ctrl_h(&self) -> f32 {
        if self.ui_ctrl_wrapped() {
            self.scale.s(72.0)
        } else {
            self.scale.s(36.0)
        }
    }

    /// The UI-controls row wraps to 2+1 rows (two sliders, the third
    /// dropping to a second row below) when the dashboard surface is too
    /// narrow to keep all three sliders comfortably side by side.
    pub(crate) fn ui_ctrl_wrapped(&self) -> bool {
        self.dash_w < self.scale.s(400.0)
    }

    /// Top Y of the edit-mode UI controls row ("pad / win / card" steppers +
    /// the reset chip): the TOP chrome row — directly below the banner-chips
    /// parking tray, ABOVE the strip-editor row ("Enable dashboard cards" /
    /// "Enable banner chips" toggles).
    pub(crate) fn ui_ctrl_top(&self) -> f32 {
        let th = self.banner_tray_h();
        if th > 0.0 {
            self.banner_tray_top() + th + self.tray_gap()
        } else {
            self.banner_tray_top()
        }
    }

    /// Top Y of the banner chip strip (BASE_BANNER_Y, scaled).
    pub(crate) fn banner_strip_top(&self) -> f32 {
        self.scale.s(BASE_BANNER_Y)
    }

    /// Height of the banner chip strip (BASE_BANNER_H, scaled).
    pub(crate) fn banner_strip_h(&self) -> f32 {
        self.scale.s(BASE_BANNER_H)
    }

    /// Default grid span of a card (used when dragging it OUT of the tray —
    /// parked cards don't carry a span).
    pub(crate) fn default_span(card: DashCard) -> (u16, u16) {
        // cards not in the default lattice don't auto-surface on every board;
        // give each a sensible spawn size anyway
        match card {
            DashCard::Cpu => return (5, 4),
            DashCard::ClipImg => return (5, 4),
            DashCard::AppShortcut => return (5, 4),
            _ => {}
        }
        default_dash_layout()
            .iter()
            .find(|l| l.card == card)
            .map(|l| (l.w, l.h))
            .unwrap_or((8, 6))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_shell() -> Shell {
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        Shell::new(cfg)
    }

    #[test]
    fn whole_strip_horizontal_scroll_when_narrow_panel() {
        let mut shell = sample_shell();
        shell.mode = Mode::Expanded;
        // default strip has NO L/C/R zone markers — many chips on a narrow
        // panel must overflow into a single horizontally-scrollable row
        shell.banner_order = DEFAULT_BANNER_ORDER.iter().copied().map(BannerToken::Chip).collect();
        let row_w = 320.0;
        shell.strip_row_w = row_w;
        assert!(shell.banner_strip_overflow(), "default chips overflow a 320px panel");
        let cells0 = shell.banner_cells(row_w);
        let (_, (fx0, _)) = cells0
            .iter()
            .enumerate()
            .find(|(_, c)| c.is_some())
            .map(|(i, c)| (i, c.unwrap()))
            .unwrap();
        assert!((fx0 - shell.scale.s(10.0)).abs() < 0.001, "first chip at the left margin");
        assert!(shell.banner_strip_scroll_by(4), "wheel scrolls the strip");
        let before = shell.banner_strip_scroll;
        assert!(before > 0.0);
        let cells1 = shell.banner_cells(row_w);
        let (_, (fx1, _)) = cells1
            .iter()
            .enumerate()
            .find(|(_, c)| c.is_some())
            .map(|(i, c)| (i, c.unwrap()))
            .unwrap();
        assert!(fx1 < fx0 - 0.5, "cells pan left by the scroll offset");
        assert!(shell.banner_strip_scroll_by(-400), "scrolling back works");
        assert_eq!(shell.banner_strip_scroll, 0.0, "resets at the start");
    }

    #[test]
    fn no_strip_scroll_when_content_fits() {
        let mut shell = sample_shell();
        shell.mode = Mode::Expanded;
        shell.banner_order = vec![BannerToken::Chip(BannerItem::Clock)];
        let row_w = 800.0;
        shell.strip_row_w = row_w;
        assert!(!shell.banner_strip_overflow(), "one chip fits an 800px panel");
        assert!(!shell.banner_strip_scroll_by(3), "no overflow → wheel not consumed");
    }
}
