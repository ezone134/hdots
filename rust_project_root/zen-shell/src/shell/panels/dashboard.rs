//! The dashboard: metro-tile grid, edit mode (move / resize / tray).

use super::*;
use super::layout::Layout;

/// Downward optical bias (scaled px) for text chips on the banner strip.
/// Lowercase text lines read "high" next to full-height icon glyphs even when
/// both share the same center — tune to taste (+down / −up).
pub const TEXT_Y_BIAS: f32 = 0.75;

impl Shell {
        pub(crate) fn layout_expanded(&mut self, v: &mut Vec<Cmd>, w: f32, _h: f32, pal: &Pal) {
        // `w` IS the auto-trimmed canvas width (target_size derives it from
        // the packed content) — the whole layout follows it
        // ── metro grid: cards are N×M spans on a fine 20×12 cell lattice.
        // Positions are DERIVED by the flow packer every frame — resizing or
        // moving one card pushes the others aside live, and the canvas
        // AUTO-FITS the content: grows when a drag pushes past the edge,
        // trims back to exactly what the cards use on commit.

        self.dash_w = w;
        self.dash_h = _h;
        let lt = Layout::compute(self, w, _h);
        let editing = lt.editing;
        // EDIT MODE: an opaque backdrop from the top edge down to the grid
        // top hides whatever sits behind the edit chrome (banner parking
        // tray / strip-editor row / cards tray are the only chrome above
        // the board). Top corners follow the surface shaping.
        if editing {
            v.push(Cmd::RectConcave {
                x: 0.0,
                y: 0.0,
                w,
                h: lt.grid_top,
                r_tl: lt.cr,
                r_tr: lt.cr,
                r_br: 0.0,
                r_bl: 0.0,
                color: lt.opaque,
            });
        }
        // cache the row width for the banner strip (fillers split it) so
        // input hit-testing and the drawer can never disagree
        self.strip_row_w = w;
        // During edit drag, edit_motion already computed the pack into
        // edit_preview_pack — sync it to packed_layout without the full
        // refresh_pack overhead (avoids HashSet rebuild for committed pack).
        if let Some((ref preview, _)) = self.edit_preview_pack {
            self.dash_cols = preview.iter().map(|l| l.x + l.w).max().unwrap_or(1).clamp(1, self.grid_max_cols());
            self.dash_rows = preview.iter().map(|l| l.y + l.h).max().unwrap_or(1).max(1);
            self.packed_layout = preview.clone();
        } else if self.edit_resize.is_none() && self.edit_drag.is_none() {
            self.refresh_pack();
        }
        // the surface is viewport-capped to the monitor — keep the pan within
        // what's actually visible and translate all content-space drawing
        self.clamp_scroll();
        // auto-follow the singing lyric line (keeps the active row on screen
        // while playing) — cheap enough to run every redraw
        if !self.lyrics_lines.is_empty() {
            self.lyrics_follow(self.lyrics_card_visible());
        }
        let layouts = self.packed_layout.clone();
        // scene values are built ONCE per frame and shared by every card draw
        // (they used to be re-formatted per card) — and skipped entirely when
        // no card actually on screen has a `.ron` scene (pure Rust-card board)
        let vals = {
            let mut scene_on = layouts.iter().any(|l| self.scene_cache.get(l.card.id()).is_some());
            if !scene_on {
                scene_on = match self.edit_drag {
                    Some((c, _, _)) => self.scene_cache.get(c.id()).is_some(),
                    None => false,
                };
            }
            // the declarative `dashboard` surface (banner band + chips) also
            // needs live values even on a pure-Rust card board
            if !scene_on && !lt.editing && !self.sys_menu_open && self.shell_scene.get().map_or(false, |ss| ss.surface("dashboard").is_some()) {
                scene_on = true;
            }
            let mut vals = if scene_on {
                self.scene_values()
            } else {
                crate::scene::SceneValues::new()
            };
            // viewing state: hand the strip cells to the declarative BannerRow
            // (sets dash_chips_via_scene so the Rust chip painter stands down)
            if !lt.editing && !self.sys_menu_open {
                self.publish_banner_cells(&mut vals, w);
            } else {
                self.dash_chips_via_scene = false;
            }
            vals
        };
        // settle-glide state for this pass (app keeps rendering while true)
        self.edit_settling = false;

        // fluid edit drags: the MOVED card rides 1:1 under the cursor, and
        // every other card GLIDES to its re-packed slot instead of snapping
        // (used by the board painter + every edit-chrome row below)
        let cursor_px = if lt.editing { self.cursor } else { None };
        let drag_state = if lt.editing { self.edit_drag } else { None };

        // ── the board (card grid) ──
        // VIEWING STATE: consult the declared `dashboard` surface. The board
        // (`Ink("dash_grid")`) + top banner strip (`Ink("dash_banner")`) route
        // through the exact same Rust painters, and the author's declared
        // chrome can layer between them. Edit-mode chrome stays Rust (its
        // geometry is frame-dynamic) and the surface is skipped while the
        // system-card menu is open — neither is ever dropped.
        if !lt.editing && !self.sys_menu_open {
            // edit-only chrome caches stay zeroed in the viewing path so input
            // can never hit invisible tray/ctrl panels.
            self.tray_rect = (0.0, 0.0, 0.0, 0.0);
            self.tray_content_h = 0.0;
            self.banner_tray_rect = (0.0, 0.0, 0.0, 0.0);
            self.banner_ctrl_rect = (0.0, 0.0, 0.0, 0.0);
            self.banner_w_slider_rect = (0.0, 0.0, 0.0, 0.0);
            let ctx = crate::shell::panels::DashDrawCtx { lt: &lt, cursor: cursor_px, vals: &vals };
            // the surface owns the viewing strip band (declared after the
            // dash_grid Ink — it rides dash_banner's dispatch slot). The flag
            // MUST be set BEFORE the consult: the dash_grid Ink runs the grid
            // painter, which skips its own band while the flag is up.
            self.dash_band_via_scene = true;
            if self.draw_shell_surface("dashboard", v, w, _h, pal, &vals, Some(&ctx)) {
                self.layout_dash_scrollbars(v, w, _h, &lt, pal);
                return;
            }
            self.dash_band_via_scene = false;
        }
        self.layout_dash_grid(v, w, _h, &lt, cursor_px, &vals, pal);

        if editing {
        // Z-ORDER LAYER 3: chrome mask — re-covers any cards that scrolled
        // above the chrome zone. Backdrop (layer 1) was drawn before cards;
        // this mask (layer 3) is drawn after cards. (The VIEWING strip band
        // draws inside `layout_dash_grid`, keeping the same z-position for
        // both the surface and fallback paths.)
        lt.draw_chrome_mask(v, w);

            // ── banner-chips parking tray (top strip): chips that are NOT on
            // the banner strip live here. Click one to append it to the strip
            // (edit_press adds it to `banner_order`). Collapsed when the tray
            // has no unused chips (only seps / filler), giving the cards area
            // the full space.
            if !self.banner_chips_absent() {
            let btray_w = w - self.scale.s(28.0);
            let btr = (self.scale.s(14.0), lt.tray_top, btray_w, lt.banner_tray_h);
            self.banner_tray_rect = btr;
            // full-width opaque slab behind the rounded panel: any board card
            // scrolled up under this row is fully hidden (corners + side
            // gutters included) — the parked chrome always sits on the cards
            v.push(Cmd::Rect { x: 0.0, y: btr.1 - self.scale.s(2.0), w, h: btr.3 + self.scale.s(4.0), r: 0.0, color: (pal.bg & 0xffffff00) | 0xff });
            v.push(Cmd::Rect {
                x: btr.0 - self.scale.s(2.0),
                y: btr.1 - self.scale.s(2.0),
                w: btr.2 + self.scale.s(4.0),
                h: btr.3 + self.scale.s(4.0),
                r: self.scale.s(16.0),
                color: (pal.bg & 0xffffff00) | 0xff,
            });
            ui::text(
                v,
                btr.0 + self.scale.s(12.0),
                btr.1 + self.scale.s(8.0),
                ICON_WIDGETS,
                self.scale.fs(11.0),
                pal.fg,
                true,
            );
            let strip_chip_empty = !self.banner_order.iter().any(|t| matches!(t, BannerToken::Chip(_)));
            let tray_title = if strip_chip_empty {
                "Banner chips — no strip chips: click to add (banner hides until filled)"
            } else {
                "Banner chips — click to add to the strip"
            };
            ui::text(
                v,
                btr.0 + self.scale.s(28.0),
                btr.1 + self.scale.s(9.0),
                tray_title,
                self.scale.fs(9.5),
                pal.fg,
                false,
            );
            {
                let parked = self.parked_banner_tokens();
                // the chip being dragged out hides from the tray (its ghost
                // follows the cursor)
                let drag_out = self.banner_drag.as_ref().and_then(|d| if d.from_tray { Some(d.item.clone()) } else { None });
                for (j, token) in parked.iter().enumerate() {
                    if !self.banner_tray_row_vis(j) { continue; }
                    if let Some(out) = &drag_out {
                        if out == token {
                            continue;
                        }
                    }
                    let (cx, cy, cw2, ch2) = self.banner_tray_chip_px(j);
                    let hov = cursor_px
                        .map(|(px, py)| Self::in_rect(px, py, (cx, cy, cw2, ch2)))
                        .unwrap_or(false);
                    v.push(Cmd::Rect {
                        x: cx,
                        y: cy,
                        w: cw2,
                        h: ch2,
                        r: 0.0,
                        color: if hov { ui::hover_hl(&pal) } else { mix(ui::hover(&pal), pal.fg, 0.06) },
                    });
                    let lbl = match token {
                        BannerToken::Chip(item) => item.name().to_string(),
                        BannerToken::Sep(g) => g.clone(),
                        BannerToken::Filler => "spacer".to_string(),
                        BannerToken::Zone(z) => match z {
                            1 => "center",
                            2 => "right",
                            _ => "left",
                        }
                        .to_string(),
                    };
                    // parked-tray chips show their NAME (date, clock, wifi...),
                    // never the icon glyph — words read at a glance next to the
                    // "+" add badge and the strip chips they'll become
                    ui::text_c(v, cx + cw2 / 2.0, cy + ch2 / 2.0 - self.scale.s(1.0), lbl, self.scale.fs(if matches!(token, BannerToken::Chip(_)) { 11.5 } else { 10.5 }), if hov { pal.fg } else { pal.fg }, false);
                    // "+" corner badge — click-to-add affordance
                    let bd = self.scale.s(14.0);
                    v.push(Cmd::Rect {
                        x: cx + cw2 - bd + self.scale.s(2.0),
                        y: cy - self.scale.s(2.0),
                        w: bd,
                        h: bd,
                        r: 0.0,
                        color: if hov { pal.acc } else { mix(ui::hover(&pal), pal.fg, 0.10) },
                    });
                    ui::text_c(v, cx + cw2 - bd + self.scale.s(2.0) + bd / 2.0, cy + self.scale.s(1.0), "+", self.scale.fs(9.0), if hov { 0xffffff } else { pal.fg }, true);
                    self.region(cx, cy, cw2, ch2, BANNER_TRAY_KEY_BASE + j as u32);
                }
                // "clear all" — NOT a chip: a compact red pill on the tray's
                // TITLE row (the row above the click-to-add chips), pinned
                // to its right end. ONE CLICK empties the whole strip back
                // into the parked tray. Drawn after the parked chips +
                // registered LAST so its region wins.
                let bh2 = self.scale.s(16.0);
                let bcw2 = "clear all".chars().count() as f32 * 0.62 * self.scale.fs(9.0) + self.scale.s(14.0);
                let bcx = btr.0 + btr.2 - self.scale.s(12.0) - bcw2;
                let bcy = btr.1 + self.scale.s(6.0);
                let bcl_hold = self.banner_clear_all_hold.is_some();
                let bcl_pct = if bcl_hold {
                    (self.banner_clear_all_hold.unwrap().elapsed().as_secs_f32() / 3.0).min(1.0)
                } else {
                    0.0
                };
                let bcl_hov = cursor_px
                    .map(|(px, py)| Self::in_rect(px, py, (bcx, bcy, bcw2, bh2)))
                    .unwrap_or(false)
                    || (bcl_hold && self.hover_key == BANNER_CLEAR_ALL_KEY);
                v.push(Cmd::Rect {
                    x: bcx,
                    y: bcy,
                    w: bcw2,
                    h: bh2,
                    r: 0.0,
                    color: if bcl_hov { mix(RED, ui::hover(&pal), 0.6) } else { mix(RED, pal.bg, 0.55) },
                });
                if bcl_hold && bcl_pct > 0.0 {
                    let fw = (bcw2 * bcl_pct).clamp(self.scale.s(2.0), bcw2);
                    v.push(Cmd::Rect {
                        x: bcx,
                        y: bcy,
                        w: fw,
                        h: bh2,
                        r: 0.0,
                        color: mix(RED, pal.fg, 0.45),
                    });
                    if bcl_pct >= 1.0 {
                        v.push(Cmd::Rect { x: bcx, y: bcy, w: bcw2, h: bh2, r: 0.0, color: RED });
                    }
                }
                ui::text_c(v, bcx + bcw2 / 2.0, bcy + (bh2 - self.scale.fs(9.0)) / 2.0, "clear all", self.scale.fs(9.0), pal.fg, false);
                self.region(bcx, bcy, bcw2, bh2, BANNER_CLEAR_ALL_KEY);
            }
            // banner-tray scrollbar indicator (edit mode, visible when overflow)
            if self.banner_tray_overflow() {
                let tw = self.scale.s(4.0);
                let tx = btr.0 + btr.2 - self.scale.s(4.0) - tw;
                let rows = self.banner_tray_rows() as f32;
                let visible = BANNER_TRAY_VROWS as f32;
                let row_h = self.scale.s(44.0) + self.scale.s(10.0); // ch + gap
                let content_h = rows * row_h;
                let view_h = visible * row_h;
                let thumb_h = ((view_h / content_h) * view_h).max(14.0);
                let scroll_frac = if rows <= visible { 0.0 } else { (self.banner_tray_scroll_row as f32 / (rows - visible).max(1.0)).clamp(0.0, 1.0) };
                let ty_min = btr.1 + 34.0;
                let ty_max = btr.1 + btr.3 - self.scale.s(4.0) - thumb_h;
                let ty = ty_min + (ty_max - ty_min).max(0.0) * scroll_frac;
                let c = mix(pal.fg, ui::hover(&pal), 0.10);
                v.push(Cmd::Rect { x: tx, y: ty_min, w: tw, h: (ty_max - ty_min + thumb_h).max(thumb_h), r: 2.0, color: mix(c, pal.fg, 0.18) });
                v.push(Cmd::Rect { x: tx, y: ty, w: tw, h: thumb_h, r: 2.0, color: mix(c, pal.fg, 0.50) });
            }
            } else {
                // tray collapsed — no chips to hit
                self.banner_tray_rect = (0.0, 0.0, 0.0, 0.0);
            }

            // ── strip-editor row: turn the dashboard cards on/off and
            // (dashboard OFF only) nudge the banner width. Always visible in
            // edit mode, directly below the banner-chips tray. Separators and
            // the smart filler no longer have buttons here — they park on the
            // banner-chips tray above.
            self.tray_rect = (0.0, 0.0, 0.0, 0.0);
            self.tray_content_h = 0.0;
            let ctrl = (self.scale.s(14.0), lt.banner_ctrl_top, w - self.scale.s(28.0), lt.banner_ctrl_h);
            self.banner_ctrl_rect = ctrl;
            v.push(Cmd::Rect { x: 0.0, y: ctrl.1, w, h: ctrl.3, r: 0.0, color: (pal.bg & 0xffffff00) | 0xff });
            v.push(Cmd::Rect {
                x: ctrl.0,
                y: ctrl.1,
                w: ctrl.2,
                h: ctrl.3,
                r: self.scale.s(14.0),
                color: (pal.bg & 0xffffff00) | 0xff,
            });
            let btn_h = self.scale.s(24.0);
            let by = ctrl.1 + (ctrl.3 - btn_h) / 2.0;
            let fs = self.scale.fs(9.5);
            let txw = |s: &str| s.chars().count() as f32 * 0.62 * fs;
            let gap = self.scale.s(8.0);
            let x = ctrl.0 + self.scale.s(10.0);
            // left: dashboard on/off toggle (state-aware label); accent
            // highlights the "turn it on" action
            let (dash_lbl, dash_accent) = if self.dash_enabled {
                ("Enable dashboard cards: ON", false)
            } else {
                ("Enable dashboard cards: OFF", true)
            };
            let dash_w = txw(dash_lbl) + self.scale.s(28.0);
            self.draw_edit_lbl_btn(v, x, by, dash_w, btn_h, BANNER_CTRL_KEY_BASE, dash_lbl, cursor_px, pal, dash_accent);
            // right: enable banner chips toggle (always shown — the mirror
            // of the dashboard toggle). Off hides every item on the top
            // banner strip; the band collapses and only cards / chrome stay.
            let (chips_lbl, chips_accent) = if self.banner_chips_enabled {
                ("Enable banner chips: ON", false)
            } else {
                ("Enable banner chips: OFF", true)
            };
            let chips_w = txw(chips_lbl) + self.scale.s(28.0);
            let chips_x0 = ctrl.0 + ctrl.2 - self.scale.s(10.0) - chips_w;
            self.draw_edit_lbl_btn(v, chips_x0, by, chips_w, btn_h, BANNER_CHIPS_TOGGLE_KEY, chips_lbl, cursor_px, pal, chips_accent);
            // card-header visibility toggles — right-anchored beside the
            // banner-chips toggle (dashboard ON only: nothing to label when
            // the board is hidden). Off hides every card's header title /
            // glyph; both default ON. Short labels keep the row uncluttered.
            let mut head_x = chips_x0;
            if self.dash_enabled {
                let (glyph_lbl, glyph_acc) = if self.card_show_glyph {
                    ("Card glyphs: ON", false)
                } else {
                    ("Card glyphs: OFF", true)
                };
                let glyph_w = txw(glyph_lbl) + self.scale.s(20.0);
                head_x -= glyph_w + gap;
                self.draw_edit_lbl_btn(v, head_x, by, glyph_w, btn_h, CARD_GLYPH_TOGGLE_KEY, glyph_lbl, cursor_px, pal, glyph_acc);
                let (title_lbl, title_acc) = if self.card_show_title {
                    ("Card titles: ON", false)
                } else {
                    ("Card titles: OFF", true)
                };
                let title_w = txw(title_lbl) + self.scale.s(20.0);
                head_x -= title_w + gap;
                self.draw_edit_lbl_btn(v, head_x, by, title_w, btn_h, CARD_TITLE_TOGGLE_KEY, title_lbl, cursor_px, pal, title_acc);
            }
            // banner WIDTH control — only when the dashboard is off (on it
            // the strip spans the whole dashboard). "width auto" fits the
            // panel to its strip content and HIDES the slider (only a px
            // readout); toggle it off to unlock a draggable slider that sets
            // the panel length in px.
            if !self.dash_enabled {
                let sx = x + dash_w + gap;
                let right_edge = chips_x0 - gap;
                if self.banner_w == 0 {
                    // auto: right-anchored toggle + fitted-size readout
                    self.banner_w_slider_rect = (0.0, 0.0, 0.0, 0.0);
                    let auto_lbl = "width auto";
                    let tbw = txw(auto_lbl) + self.scale.s(18.0);
                    let tx0 = right_edge - tbw;
                    self.draw_edit_lbl_btn(v, tx0, by, tbw, btn_h, BANNER_W_AUTO_KEY, auto_lbl, cursor_px, pal, true);
                    let eff = self.banner_strip_w();
                    ui::text_r(v, tx0 - gap, by + self.scale.s(7.0), &format!("{} px", eff.round() as i32), fs, pal.fg, false);
                } else {
                    // manual: slider + live px readout + right-anchored toggle
                    let manual_lbl = "width manual";
                    let tbw = txw(manual_lbl) + self.scale.s(18.0);
                    let tx0 = right_edge - tbw;
                    self.draw_edit_lbl_btn(v, tx0, by, tbw, btn_h, BANNER_W_AUTO_KEY, manual_lbl, cursor_px, pal, false);
                    let sw = (tx0 - gap - sx).max(self.scale.s(60.0));
                    let val = (self.banner_w as f32 / 6000.0).clamp(0.0, 1.0);
                    let sy = by + (btn_h - self.scale.s(10.0)) / 2.0;
                    let hovered = cursor_px.map(|(px, py)| Self::in_rect(px, py, (sx, sy, sw, self.scale.s(10.0)))).unwrap_or(false);
                    ui::slider(v, sx, sy, sw, self.scale.s(10.0), val, hovered, pal);
                    self.region(sx, sy, sw, self.scale.s(10.0), BANNER_W_SLIDER_KEY);
                    self.banner_w_slider_rect = (sx, sy, sw, self.scale.s(10.0));
                    ui::text_r(v, sx + sw + gap, by + self.scale.s(7.0), &format!("{} px", self.banner_w), fs, pal.fg, false);
                }
            } else {
                self.banner_w_slider_rect = (0.0, 0.0, 0.0, 0.0);
            }
            // ── dashboard CARDS parking tray: cards parked here live OFF the
            // grid. Drag a card here (or hit its ✕) to remove it; click a
            // chip to place it on grid. Hidden when the dashboard is off OR
            // collapsed (no cards at all — the edit view then shows only the
            // banner chips area).
            if self.dash_enabled && !self.dash_area_collapsed() {
            let cw_chip = self.scale.s(96.0);
            let ch_chip = self.scale.s(44.0);
            let tray_w = w - self.scale.s(28.0);
            let per_row = ((tray_w - self.scale.s(24.0)) / (cw_chip + self.scale.s(12.0))).floor().max(1.0) as usize;
            let chip_rows = if self.tray_cards.is_empty() {
                0
            } else {
                ((self.tray_cards.len() + per_row - 1) / per_row).max(1)
            };
            let visible_chip_rows = chip_rows.min(DASH_TRAY_VROWS);                let content_h = if visible_chip_rows == 0 {
                    self.scale.s(44.0)
                } else {
                    self.scale.s(34.0) + visible_chip_rows as f32 * (ch_chip + self.scale.s(10.0)) + self.scale.s(8.0)
                };
            self.tray_content_h = content_h;
            let tr = (self.scale.s(14.0), lt.banner_ctrl_top + lt.banner_ctrl_h + lt.tray_gap, tray_w, content_h);
            self.tray_rect = tr;
            let drag_live = drag_state.is_some();
            let over = cursor_px
                .map(|(px, py)| Self::in_rect(px, py, tr))
                .unwrap_or(false);
            // tray background — elevated panel feel, drawn over an opaque
            // full-width slab so scrolled board cards never bleed through
            v.push(Cmd::Rect { x: 0.0, y: tr.1 - self.scale.s(2.0), w, h: tr.3 + self.scale.s(4.0), r: 0.0, color: (pal.bg & 0xffffff00) | 0xff });
            v.push(Cmd::Rect {
                x: tr.0 - self.scale.s(2.0),
                y: tr.1 - self.scale.s(2.0),
                w: tr.2 + self.scale.s(4.0),
                h: tr.3 + self.scale.s(4.0),
                r: self.scale.s(16.0),
                color: (pal.bg & 0xffffff00) | 0xff,
            });
            // divider line between tray and card grid below
            let div_y = tr.1 + tr.3 + self.scale.s(6.0);
            v.push(Cmd::Rect {
                x: lt.pad,
                y: div_y,
                w: w - lt.pad * 2.0,
                h: self.scale.s(1.0),
                r: self.scale.s(0.5),
                color: mix(ui::hover(&pal), pal.fg, 0.18),
            });
            if drag_live && over {
                // drop target highlight — tinted background instead of shadow
                v.push(Cmd::Rect {
                    x: tr.0 - self.scale.s(2.0),
                    y: tr.1 - self.scale.s(2.0),
                    w: tr.2 + self.scale.s(4.0),
                    h: tr.3 + self.scale.s(4.0),
                    r: self.scale.s(16.0),
                    color: (pal.acc & 0xFFFF_FF00) | 0x18,
                });
            }
            // header: icon + label
            ui::text(
                v,
                tr.0 + self.scale.s(12.0),
                tr.1 + self.scale.s(8.0),
                ICON_ARRANGE,
                self.scale.fs(11.0),
                pal.fg,
                true,
            );
            ui::text(
                v,
                tr.0 + self.scale.s(28.0),
                tr.1 + self.scale.s(9.0),
                "Dashboard cards",
                self.scale.fs(9.5),
                pal.fg,
                false,
            );
            // edit-mode grid-size toolbar (rows/cols counts + "+"/"-" buttons)
            // — right-anchored on the tray header; grows/shrinks the whole
            // board so the dashboard gets taller/wider
            let fs = self.scale.fs(9.5);
            let bw = self.scale.s(18.0);
            let bh = self.scale.s(20.0);
            let by = tr.1 + self.scale.s(3.0);
            let gap = self.scale.s(6.0);
            let cw_t = |s: &str| s.chars().count() as f32 * 0.62 * fs + 2.0;
            let rows_s = format!("rows {}", self.canvas_rows());
            let cols_s = format!("cols {}", self.canvas_cols());
            let clear_lbl = "Clear all";
            let clear_w = cw_t(clear_lbl) + self.scale.s(16.0);
            let tw = bw * 2.0; // dual-glyph stack toggle: [↕ | ↔]
            let cluster_w = clear_w
                + self.scale.s(14.0)
                + cw_t("stack") + cw_t(&rows_s) + cw_t(&cols_s)
                + (bw + gap) * 4.0 + tw + gap
                + self.scale.s(14.0) * 2.0;
            let mut lx = (tr.0 + tr.2 - self.scale.s(8.0) - cluster_w).max(tr.0 + self.scale.s(8.0));
            // clear all cards button (ONE CLICK clears the board at release)
            // — leftmost, before the scroll / width cluster
            let clear_holding = self.dash_clear_all_hold.is_some();
            let clear_pct = if clear_holding {
                let elapsed = self.dash_clear_all_hold.unwrap().elapsed().as_secs_f32();
                (elapsed / 3.0).min(1.0)
            } else {
                0.0
            };
            let clear_hov = cursor_px
                .map(|(px, py)| Self::in_rect(px, py, (lx, by, clear_w, bh)))
                .unwrap_or(false);
            // background
            let clear_bg = if clear_hov {
                mix(RED, ui::hover(&pal), 0.6)
            } else {
                mix(ui::hover(&pal), pal.fg, 0.10)
            };
            v.push(Cmd::Rect { x: lx, y: by, w: clear_w, h: bh, r: 0.0, color: clear_bg });
            // liquid fill-up: the WHOLE button fills left→right while held
            if clear_holding && clear_pct > 0.0 {
                let fw = (clear_w * clear_pct).clamp(self.scale.s(1.0), clear_w);
                v.push(Cmd::Rect {
                    x: lx,
                    y: by,
                    w: fw,
                    h: bh,
                    r: 0.0,
                    color: mix(RED, pal.bg, 0.35),
                });
                if clear_pct >= 1.0 {
                    v.push(Cmd::Rect { x: lx, y: by, w: clear_w, h: bh, r: 0.0, color: RED });
                }
            }
            ui::text_c(v, lx + clear_w / 2.0, by + self.scale.s(3.5), clear_lbl, self.scale.fs(9.5), pal.fg, false);
            self.region(lx, by, clear_w, bh, DASH_CLEAR_ALL_KEY);
            lx += clear_w + self.scale.s(14.0);
            // scroll direction toggle — ONE button that flips wheel-scroll rows
            // ↔ columns, showing BOTH direction glyphs side by side (↕ rows /
            // ↔ columns): the active direction is accent-highlighted, the idle
            // one dimmed, so the current state reads at a glance.
            ui::text(v, lx, by + self.scale.s(3.5), "stack", fs, pal.fg, false);
            lx += cw_t("stack");
            let vert_active = !self.dash_scroll_dir;
            self.draw_edit_toggle_btn(v, lx, by, tw, bh, DASH_SCROLL_DIR_KEY, ICON_SWAP_VERT, ICON_SWAP_HORIZ, vert_active, cursor_px, pal);
            lx += tw + self.scale.s(14.0);
            // rows 27 [−][+]
            ui::text(v, lx, by + self.scale.s(3.5), &rows_s, fs, pal.fg, false);
            lx += cw_t(&rows_s);
            let rows_can_dec = self.canvas_rows() > 1;
            let rows_can_inc = self.canvas_rows() < self.rows_cap();
            let cols_can_dec = self.canvas_cols() > 1;
            let cols_can_inc = self.canvas_cols() < self.edit_cols_cap();
            self.draw_edit_btn(v, lx, by, bw, bh, 504, "-", cursor_px, pal, rows_can_dec);
            lx += bw + gap;
            self.draw_edit_btn(v, lx, by, bw, bh, 505, "+", cursor_px, pal, rows_can_inc);
            lx += bw + self.scale.s(14.0);
            // cols 20 [−][+]
            ui::text(v, lx, by + self.scale.s(3.5), &cols_s, fs, pal.fg, false);
            lx += cw_t(&cols_s);
            self.draw_edit_btn(v, lx, by, bw, bh, 506, "-", cursor_px, pal, cols_can_dec);
            lx += bw + gap;
            self.draw_edit_btn(v, lx, by, bw, bh, 507, "+", cursor_px, pal, cols_can_inc);
            for (i, card) in self.tray_cards.clone().iter().enumerate() {
                // pager: only draw chips in the visible row window
                if !self.dash_tray_row_vis(i) { continue; }
                // the chip being dragged floats with the cursor instead
                if matches!(drag_state, Some((c, _, _)) if c == *card) {
                    continue;
                }
                let (cx, cy, cw2, ch2) = self.tray_chip_px(i);
                let hov = cursor_px
                    .map(|(px, py)| Self::in_rect(px, py, (cx, cy, cw2, ch2)))
                    .unwrap_or(false);
                // chip background
                v.push(Cmd::Rect {
                    x: cx,
                    y: cy,
                    w: cw2,
                    h: ch2,
                    r: 0.0,
                    color: if hov { ui::hover_hl(&pal) } else { mix(ui::hover(&pal), pal.fg, 0.06) },
                });
                ui::text_c(v, cx + cw2 / 2.0, cy + ch2 / 2.0 - self.scale.s(7.0), Self::card_name(*card), self.scale.fs(11.5), if hov { pal.fg } else { pal.fg }, false);
                ui::text_c(v, cx + cw2 / 2.0, cy + ch2 / 2.0 + self.scale.s(9.0), "drag →", self.scale.fs(8.5), pal.fg, false);
                // "+" corner badge — click-to-place affordance: clicking the
                // chip puts the card on the grid at the next free slot
                let badge_d = self.scale.s(16.0);
                v.push(Cmd::Rect {
                    x: cx + cw2 - badge_d + self.scale.s(4.0),
                    y: cy - self.scale.s(2.0),
                    w: badge_d,
                    h: badge_d,
                    r: 0.0,
                    color: if hov { pal.acc } else { mix(ui::hover(&pal), pal.fg, 0.10) },
                });
                ui::text_c(v, cx + cw2 - badge_d + self.scale.s(4.0) + badge_d / 2.0, cy + self.scale.s(2.0), "+", self.scale.fs(9.0), if hov { 0xffffff } else { pal.fg }, true);
            }
            // parked-cards tray scrollbar indicator (edit mode, visible when overflow)
            if self.dash_tray_overflow() {
                let tw = self.scale.s(4.0);
                let tx = tr.0 + tr.2 - self.scale.s(4.0) - tw;
                let visible = DASH_TRAY_VROWS as f32;
                let row_h = ch_chip + self.scale.s(10.0);
                let content_h_abs = chip_rows as f32 * row_h;
                let view_h = visible * row_h;
                let thumb_h = ((view_h / content_h_abs) * view_h).max(14.0);
                let scroll_frac = if chip_rows as f32 <= visible { 0.0 } else { (self.dash_tray_scroll_row as f32 / (chip_rows as f32 - visible).max(1.0)).clamp(0.0, 1.0) };
                let ty_min = tr.1 + 26.0;
                let ty_max = tr.1 + tr.3 - self.scale.s(4.0) - thumb_h;
                let ty = ty_min + (ty_max - ty_min).max(0.0) * scroll_frac;
                let c = mix(pal.fg, ui::hover(&pal), 0.10);
                v.push(Cmd::Rect { x: tx, y: ty_min, w: tw, h: (ty_max - ty_min + thumb_h).max(thumb_h), r: 2.0, color: mix(c, pal.fg, 0.18) });
                v.push(Cmd::Rect { x: tx, y: ty, w: tw, h: thumb_h, r: 2.0, color: mix(c, pal.fg, 0.50) });
            }
            } // if self.dash_enabled — cards tray hidden when dashboard is off
        } else {
            self.tray_rect = (0.0, 0.0, 0.0, 0.0);
            self.tray_content_h = 0.0;
            self.banner_tray_rect = (0.0, 0.0, 0.0, 0.0);
            self.banner_ctrl_rect = (0.0, 0.0, 0.0, 0.0);
        }

        // ── UI controls row: Window Padding / Window Rounding /
        // Card Rounding (linked to Window Rounding) + Reset ──────
        // EDIT MODE ONLY — sits directly above the strip-editor row
        // ("Enable dashboard cards" / "Enable banner chips" toggles).
        if editing {
            let uc_top = lt.ui_ctrl_top;
            let uc_h = lt.ui_ctrl_h;
            let pad = self.scale.s(14.0);
            let uc = (pad, uc_top, w - pad * 2.0, uc_h);
            v.push(Cmd::Rect { x: 0.0, y: uc_top - self.scale.s(2.0), w, h: uc_h + self.scale.s(4.0), r: 0.0, color: (pal.bg & 0xffffff00) | 0xff });
            v.push(Cmd::Rect {
                x: uc.0,
                y: uc.1,
                w: uc.2,
                h: uc.3,
                r: self.scale.s(12.0),
                color: (pal.bg & 0xffffff00) | 0xff,
            });
            let fs = self.scale.fs(9.0);
            let cw_t = |s: &str| s.chars().count() as f32 * 0.56 * fs;
            let gap = self.scale.s(6.0);
            let btn_h = self.scale.s(20.0);

            if !self.ui_ctrl_wrapped() {
                // ── one row: Window Padding · Window Rounding · Card
                //    Rounding (linked) · Reset ──
                let seg_w = uc.2 / 4.0; // four columns: three steppers + reset

                // ── Window Padding stepper ──
                self.draw_ui_ctrl_stepper(v, uc.0 + gap, seg_w - gap * 2.0, uc.1 + self.scale.s(2.0), uc.1 + self.scale.s(12.0), "Window Padding", &format!("{}px", self.ui_pad.round() as i32), UI_PAD_MINUS_KEY, UI_PAD_PLUS_KEY, true, cursor_px, pal);
                self.ui_pad_slider_rect = (0.0, 0.0, 0.0, 0.0);

                // ── Window Rounding stepper ──
                self.draw_ui_ctrl_stepper(v, uc.0 + seg_w + gap, seg_w - gap * 2.0, uc.1 + self.scale.s(2.0), uc.1 + self.scale.s(12.0), "Window Rounding", &format!("{}px", self.win_radius.round() as i32), WIN_RAD_MINUS_KEY, WIN_RAD_PLUS_KEY, true, cursor_px, pal);
                self.win_rad_slider_rect = (0.0, 0.0, 0.0, 0.0);

                // ── Card Rounding stepper (LINKED to the window radius:
                //    its −/+ move the same shared value) ──
                let card_readout = format!("{}px", self.win_radius.round() as i32);
                self.draw_ui_ctrl_stepper(v, uc.0 + seg_w * 2.0 + gap, seg_w - gap * 2.0, uc.1 + self.scale.s(2.0), uc.1 + self.scale.s(12.0), "Card Rounding", &card_readout, CARD_RAD_MINUS_KEY, CARD_RAD_PLUS_KEY, true, cursor_px, pal);
                self.card_rad_slider_rect = (0.0, 0.0, 0.0, 0.0);

                // ── reset (far right): padding 12 · window 16 · card = window ──
                let rst_w = cw_t("reset") + self.scale.s(14.0);
                let rst_x = (uc.0 + uc.2 - self.scale.s(8.0) - rst_w).max(uc.0 + seg_w * 3.0 + gap);
                self.draw_edit_lbl_btn(v, rst_x, uc.1 + self.scale.s(2.0), rst_w, self.scale.s(16.0), UI_CTRL_RESET_KEY, "reset", cursor_px, pal, true);
            } else {
                // ── two wrapped rows: Window Padding + Window Rounding
                //    side by side, Card Rounding + Reset below ──
                let half = uc.2 / 2.0;
                let row_h = uc.3 / 2.0;
                let ltop1 = uc.1 + self.scale.s(4.0);
                let ltop2 = uc.1 + row_h + self.scale.s(4.0);
                let row2_y = uc.1 + row_h + (row_h - btn_h) / 2.0;

                // ── Window Padding stepper (left half) ──
                self.draw_ui_ctrl_stepper(v, uc.0 + gap, half - gap * 2.0, ltop1 + self.scale.s(3.5), ltop1 + self.scale.s(15.0), "Window Padding", &format!("{}px", self.ui_pad.round() as i32), UI_PAD_MINUS_KEY, UI_PAD_PLUS_KEY, true, cursor_px, pal);
                self.ui_pad_slider_rect = (0.0, 0.0, 0.0, 0.0);

                // ── Window Rounding stepper (right half) ──
                self.draw_ui_ctrl_stepper(v, uc.0 + half + gap, half - gap * 2.0, ltop1 + self.scale.s(3.5), ltop1 + self.scale.s(15.0), "Window Rounding", &format!("{}px", self.win_radius.round() as i32), WIN_RAD_MINUS_KEY, WIN_RAD_PLUS_KEY, true, cursor_px, pal);
                self.win_rad_slider_rect = (0.0, 0.0, 0.0, 0.0);

                // ── reset (second row, far right) ──
                let rst_w = cw_t("reset") + self.scale.s(14.0);
                let rst_x = (uc.0 + uc.2 - self.scale.s(8.0) - rst_w).max(uc.0 + half / 2.0);
                self.draw_edit_lbl_btn(v, rst_x, row2_y, rst_w, btn_h, UI_CTRL_RESET_KEY, "reset", cursor_px, pal, true);

                // ── Card Rounding stepper (second row, left of reset;
                //    LINKED to the window radius) ──
                let card_readout = format!("{}px", self.win_radius.round() as i32);
                let card_w = (rst_x - gap - uc.0 - gap).max(self.scale.s(80.0));
                self.draw_ui_ctrl_stepper(v, uc.0 + gap, card_w, ltop2 + self.scale.s(3.5), ltop2 + self.scale.s(15.0), "Card Rounding", &card_readout, CARD_RAD_MINUS_KEY, CARD_RAD_PLUS_KEY, true, cursor_px, pal);
                self.card_rad_slider_rect = (0.0, 0.0, 0.0, 0.0);
            }
        }

        // ── scrollbars: overlay that only appears while scrolling or while
        // dragging the bars — then it auto-hides, fading out `DASH_SB_KEEP`
        // after the last scroll. The hit zones ALWAYS register while this axis
        // overflows, so a press on the viewport edges can still grab the bars
        // even when they're hidden.
        self.layout_dash_scrollbars(v, w, _h, &lt, pal);

        // ── top BANNER strip: drawn LAST so scrolled board slides UNDER it
        self.layout_banner(v, w, &lt, cursor_px, pal);
        // system-card ⋮ hidden menu: drawn after everything so it sits on
        // top of the board, exactly like the connectivity jump menu
        self.draw_sys_menu(v, w, &lt, pal);
    }

    /// The dashboard's scrollbar overlays (V + H rails + thumbs, fade-out
    /// alpha). Extracted from `layout_expanded` so the surface path draws them
    /// in the same z-position (the drag hit zones 500/501 always register).
    pub(crate) fn layout_dash_scrollbars(&mut self, v: &mut Vec<Cmd>, w: f32, h: f32, lt: &Layout, pal: &Pal) {
        let now = std::time::Instant::now();
        if self.dash_sb_drag.is_some() {
            self.dash_sb_last = Some(now);
        }
        let sb_a = self.dash_sb_alpha(now);
        let over_y = (lt.canvas_h - h).max(0.0);
        let over_x = (lt.canvas_w - w).max(0.0);
        // neutral translucent gray derived from fg (NOT the accent/blue tint):
        // rail ~10%, thumb ~38% fg at full reveal.
        let rail_col = (pal.fg & 0x00ffffff) | (0x19 << 24);
        let thumb_col = (pal.fg & 0x00ffffff) | (0x61 << 24);
        let sb_col = |c: u32| -> u32 {
            let a = (((c >> 24) & 0xff) as f32 * sb_a).round() as u32;
            (c & 0x00ffffff) | (a << 24)
        };
        let visible = sb_a > 0.02;
        if over_y > 1.0 {
            let tw = self.scale.s(6.0);
            let tx = w - tw - self.scale.s(4.0);
            let ty = (lt.grid_top + self.scale.s(4.0)).min(h - tw - self.scale.s(8.0));
            let th = (h - ty - self.scale.s(6.0)).max(20.0);
            if visible {
                let thumb_h = (th * (h / lt.canvas_h).min(1.0)).max(20.0);
                let tyy = ty + (th - thumb_h).max(0.0) * (self.dash_scroll_y / over_y).clamp(0.0, 1.0);
                v.push(Cmd::Rect { x: tx, y: ty, w: tw, h: th, r: self.scale.s(3.0), color: sb_col(rail_col) });
                v.push(Cmd::Rect { x: tx, y: tyy, w: tw, h: thumb_h, r: self.scale.s(3.0), color: sb_col(thumb_col) });
            }
            self.region(tx - self.scale.s(5.0), ty, tw + self.scale.s(10.0), th, 500);
        }
        // ── horizontal scroller (bottom): boards may grow wide in either
        // stack direction, so the rail shows whenever the canvas overflows
        // sideways (regardless of scroll direction).
        let h_rail = over_x > 1.0;
        if h_rail {
            let thh = self.scale.s(6.0);
            let tyy = h - thh - self.scale.s(4.0);
            let tx = (self.scale.s(6.0)).min(w - thh - self.scale.s(8.0));
            let tw = (w - self.scale.s(16.0)).max(20.0);
            if visible {
                v.push(Cmd::Rect { x: tx, y: tyy, w: tw, h: thh, r: self.scale.s(3.0), color: sb_col(rail_col) });
                if over_x > 1.0 {
                    let thumb_w = (tw * (w / lt.canvas_w).min(1.0)).max(30.0);
                    let txx = tx + (tw - thumb_w).max(0.0) * (self.dash_scroll_x / over_x).clamp(0.0, 1.0);
                    v.push(Cmd::Rect { x: txx, y: tyy, w: thumb_w, h: thh, r: self.scale.s(3.0), color: sb_col(thumb_col) });
                }
            }
            self.region(tx, tyy - self.scale.s(5.0), tw, thh + self.scale.s(10.0), 501);
        }
    }

    /// The dashboard board — the card grid body. Extracted from
    /// `layout_expanded` so the declared `dashboard` surface's `dash_grid` Ink
    /// draws exactly what the fallback does: edit dot lattice, empty hint,
    /// drag ghost, every card at its packed placement (fluid glide while
    /// editing), the tray-drag ghost, then the viewing strip band. Carries the
    /// state side effects with it (fluid `card_anim` glide, `edit_settling`,
    /// the `DASH_CARD_KEY` hit regions) so the surface path behaves identically.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn layout_dash_grid(
        &mut self,
        v: &mut Vec<Cmd>,
        w: f32,
        h: f32,
        lt: &Layout,
        cursor_px: Option<(f32, f32)>,
        vals: &crate::scene::SceneValues,
        pal: &Pal,
    ) {
        let editing = lt.editing;
        let sx = lt.scroll_x;
        let sy = lt.scroll_y;
        let cw = lt.cell_w;
        let chh = lt.cell_h;
        let drag_state = if editing { self.edit_drag } else { None };

        // EDIT MODE chrome: faint dot lattice at every cell corner. Ranges are
        // VIEWPORT-BOUNDED (scroll + surface size), so adding rows/cols never
        // rebuilds a scanned dot grid — a 512×1024 canvas costs the same as a
        // 20×12 one (instantly responsive "+"/"−" in the grid-size toolbar).
        if editing && self.dash_enabled {
            let dot = mix(ui::hover(&pal), pal.fg, 0.12);
            let px = cw + lt.gap;
            let py = chh + lt.gap;
            let gx0 = (((sx - lt.pad) / px).floor() as i64).max(0) as usize;
            let gx1 = (((sx - lt.pad + w) / px).ceil() as i64).max(0) as usize;
            let gy0 = (((sy - lt.grid_top) / py).floor() as i64).max(0) as usize;
            let gy1 = (((sy - lt.grid_top + h) / py).ceil() as i64).max(0) as usize;
            let ccol = self.canvas_cols() as usize;
            let crow = self.canvas_rows() as usize;
            for gy in gy0..=gy1.min(crow) {
                for gx in gx0..=gx1.min(ccol) {
                    let ex = lt.pad + gx as f32 * px - sx;
                    let ey = lt.grid_top + gy as f32 * py - sy;
                    if ex >= 0.0 && ex <= w - 3.0 && ey >= lt.grid_top - 1.0 && ey <= h - 3.0 {
                        v.push(Cmd::Rect { x: ex - 1.5, y: ey - 1.5, w: 3.0, h: 3.0, r: 1.5, color: dot });
                    }
                }
            }
        }

        // empty BOARD (cards may still sit parked in the tray below): a
        // gentle hint while editing
        if editing && self.dash_enabled && self.dash_layout.is_empty() {
            let hint = if self.dash_area_collapsed() {
                "no dashboard cards — everything below is collapsed; the empty board will grow as you add cards"
            } else {
                "no dashboard cards — click a chip in the tray above (or drag one out)"
            };
            ui::text_c(
                v,
                w / 2.0,
                lt.grid_top + self.scale.s(64.0),
                hint,
                self.scale.fs(11.0),
                mix(ui::hover(&pal), pal.fg, 0.55),
                false,
            );
        }
        // ghost rectangle at target slot during drag
        if editing && self.dash_enabled {
            if let (Some((card, _, _)), Some((gx, gy))) = (drag_state, self.edit_ghost_slot) {
                let span = self.dash_layout.iter().find(|l| l.card == card).map(|l| (l.w, l.h)).unwrap_or((1, 1));
                let (ghx0, ghy0, ghw, ghh) = dash_card_px(
                    CardLayout { card, x: gx, y: gy, w: span.0, h: span.1 },
                    cw, chh, lt.grid_top, lt.pad, lt.gap,
                );
                let ghx = ghx0 - sx;
                let ghy = ghy0 - sy;
                let ghost_color = (pal.acc & 0x00ffffff) | 0x30000000;
                let border_color = (pal.acc & 0x00ffffff) | 0x60000000;
                v.push(Cmd::Rect { x: ghx, y: ghy, w: ghw, h: ghh, r: lt.card_r, color: ghost_color });
                // inner border (inset 2px)
                let ins = 2.0;
                v.push(Cmd::Rect { x: ghx + ins, y: ghy + ins, w: ghw - ins * 2.0, h: ghh - ins * 2.0, r: (lt.card_r - ins).max(0.0), color: border_color });
            }
        }

        // every card at its packed placement (the dragged one included — it
        // rides along in the re-packed flow like a springboard icon)
        let packed = self.packed_layout.clone();
        if self.dash_enabled {
        for (i, l) in packed.iter().enumerate() {
            let (mut x, mut y, rw, rh) = dash_card_px(*l, cw, chh, lt.grid_top, lt.pad, lt.gap);
            let mut fluid = false;
            if let Some((card, gx, gy)) = drag_state {
                if card == l.card {
                    if let Some((mx, my)) = cursor_px {
                        x = (mx - gx).clamp(lt.pad, (lt.canvas_w - lt.pad - rw).max(lt.pad));
                        y = (my - gy).clamp(lt.grid_top, (lt.canvas_h - lt.pad - rh).max(lt.grid_top));
                        fluid = true;
                    }
                }
            }
            if editing && !fluid {
                // ease toward the packed slot — springboard-style glide
                let prev = self.card_anim.entry(l.card).or_insert((x, y));
                prev.0 += (x - prev.0) * 0.35;
                prev.1 += (y - prev.1) * 0.35;
                if (prev.0 - x).abs() > 0.4 || (prev.1 - y).abs() > 0.4 {
                    self.edit_settling = true;
                }
                x = prev.0;
                y = prev.1;
            }
            // the grid pans under the viewport — scrolled cards are drawn
            // in surface space, clipped by the surface itself
            x -= sx;
            y -= sy;
            // hover slot so crossing a card repaints (lets hover-only chrome
            // like pane dots hide/show live)
            self.region(x, y, rw, rh, DASH_CARD_KEY + i as u32);
            let active = matches!(drag_state, Some((c, _, _)) if c == l.card);
            let card_bg = (if active { ui::raised_hl(&pal) } else { ui::raised(&pal) } & 0xffffff00) | self.dash_card_alpha as u32;
            v.push(Cmd::Rect { x, y, w: rw, h: rh, r: lt.card_r, color: card_bg });
                l.card.draw(self, v, x, y, rw, rh, pal, vals);
                if editing {
                    // ✕ close button (top-right): click parks the card into the
                    // tray — follows the card like the grip does
                let xr = (x + rw - self.scale.s(27.0), y + self.scale.s(7.0), self.scale.s(20.0), self.scale.s(20.0));
                let hov_x =
                    cursor_px.map(|(px, py)| Self::in_rect(px, py, xr)).unwrap_or(false);
                v.push(Cmd::Rect {
                    x: xr.0,
                    y: xr.1,
                    w: xr.2,
                    h: xr.3,
                    r: self.scale.s(10.0),
                    color: if hov_x { mix(RED, ui::hover(&pal), 0.55) } else { mix(ui::hover(&pal), pal.fg, 0.10) },
                });
                ui::text(
                    v,
                    xr.0 + 6.5,
                    xr.1 + 3.5,
                    ICON_CLOSE_FILL,
                    self.scale.fs(11.0),
                    if hov_x { RED } else { pal.fg },
                    true,
                );
                // corner resize grip (⌟ dot triangle) — follows the card
                let gr = (x + rw - self.scale.s(18.0), y + rh - self.scale.s(18.0), 16.0, 16.0);
                let hov_grip =
                    cursor_px.map(|(px, py)| Self::in_rect(px, py, gr)).unwrap_or(false);
                let gc = if hov_grip || active { pal.acc } else { mix(ui::hover(&pal), pal.fg, 0.35) };
                for j in 0..3 {
                    for i in 0..3 {
                        if i + j > 2 {
                            continue; // triangle
                        }
                        v.push(Cmd::Rect {
                            x: gr.0 + self.scale.s(4.0) + j as f32 * self.scale.s(4.0),
                            y: gr.1 + self.scale.s(4.0) + i as f32 * self.scale.s(4.0),
                            w: self.scale.s(2.6),
                            h: self.scale.s(2.6),
                            r: self.scale.s(1.3),
                            color: gc,
                        });
                    }
                }
            }
        }

        // a card being dragged OUT of the tray isn't in the packed grid —
        // draw it floating under the cursor at its default span
        if editing {
            if let (Some((card, dx, dy)), Some((mx, my))) = (drag_state, cursor_px) {
                if self.edit_drag_from_tray {
                    let (sw, sh) = crate::shell::Shell::default_span(card);
                    let (_, _, fw, fh) =
                        dash_card_px(CardLayout { card, x: 0, y: 0, w: sw, h: sh }, cw, chh, lt.grid_top, lt.pad, lt.gap);
                    let fx = (mx - dx).clamp(lt.pad, (w - lt.pad - fw).max(lt.pad));
                    let fy = (my - dy).clamp(lt.grid_top, (h - lt.pad - fh).max(lt.grid_top));
                    v.push(Cmd::Rect { x: fx, y: fy, w: fw, h: fh, r: lt.card_r, color: ui::hover(&pal) });
                    card.draw(self, v, fx, fy, fw, fh, pal, vals);
                }
            }
        }
        }

        // the opaque strip band covers scrolled board slides under the banner
        // (viewing state) — the edit-mode chrome mask draws above the cards in
        // `layout_expanded`; this is the same z-position for the non-edit path.
        // SKIPPED while the declared `dashboard` surface owns it (no double
        // paint — the band sits between the two Inks in shell.ron).
        if !editing && !self.dash_band_via_scene && !self.banner_collapsed() {
            v.push(Cmd::RectConcave {
                x: 0.0,
                y: lt.strip_top,
                w,
                h: lt.strip_h,
                r_tl: lt.cr,
                r_tr: lt.cr,
                r_br: 0.0,
                r_bl: 0.0,
                color: lt.opaque,
            });
        }
    }

    /// The system card's ⋮ hidden menu — "Bar position" (expands to a 4-edge
    /// submenu). The ">" chevron in the card's bottom-right corner opens
    /// Settings directly (settings now live on the card, not in the menu).
    /// Anchored under the ⋮ button in the card's top-right corner; any click
    /// outside the menu closes it (the expanded-mode default arm resets
    /// `sys_menu_open`).
    fn draw_sys_menu(&mut self, v: &mut Vec<Cmd>, w: f32, lt: &Layout, pal: &Pal) {
        if !self.sys_menu_open || self.dash_edit {
            return;
        }
        let Some(&l) = self.packed_layout.iter().find(|l| l.card == DashCard::System) else {
            return;
        };
        let (x, y, rw, _) = dash_card_px(l, lt.cell_w, lt.cell_h, lt.grid_top, lt.pad, lt.gap);
        let x = x - lt.scroll_x;
        let y = y - lt.scroll_y;
        let ptw = self.scale.s(168.0);
        // right-aligned under the ⋮ button (card top-right corner)
        let ax = (x + rw - self.scale.s(14.0) - ptw)
            .clamp(self.scale.s(8.0), (w - ptw - self.scale.s(8.0)).max(self.scale.s(8.0)));
        let ay0 = y + self.scale.s(10.0) + self.scale.s(22.0) + self.scale.s(6.0);
        let row_h = self.scale.s(30.0);
        let gfs = self.scale.fs(13.0);
        let tfs = self.scale.fs(11.0);
        let vy_off = self.scale.s(1.0);
        let mar = self.scale.s(3.0);
        // snapshot so the drawer closure borrows nothing from self
        let scale = self.scale;
        let hov_key = self.hover_key;
        let row = |v: &mut Vec<Cmd>,
                   key: u32,
                   glyph: &str,
                   label: &str,
                   active: bool,
                   chevron: bool,
                   ry: f32| {
            let hov = hov_key == key;
            if hov {
                v.push(Cmd::Rect { x: ax + mar, y: ry, w: ptw - scale.s(6.0), h: row_h, r: scale.s(8.0), color: ui::hover_hl(pal) });
            }
            let gcol = if active { pal.acc } else { pal.fg };
            ui::text(v, ax + scale.s(12.0), ry + (row_h - tfs) / 2.0 + vy_off - scale.s(1.0), glyph, gfs, gcol, true);
            ui::text(v, ax + scale.s(10.0), ry + (row_h - tfs) / 2.0 + vy_off, label, tfs, if active { pal.acc } else { pal.fg }, true);
            if chevron {
                ui::text_r(v, ax + ptw - scale.s(12.0), ry + (row_h - tfs) / 2.0 + vy_off, ICON_ARROW_FORWARD, tfs, pal.sfg, true);
            }
        };

        let bp_open = self.sys_menu_bp_open;
        let rows = if bp_open { 5 } else { 1 };
        let ph = rows as f32 * row_h + self.scale.s(6.0);
        // soft shadow ring + opaque body
        v.push(Cmd::Rect {
            x: ax - self.scale.s(3.0),
            y: ay0 - self.scale.s(3.0),
            w: ptw + self.scale.s(6.0),
            h: ph + self.scale.s(6.0),
            r: self.scale.s(12.0),
            color: (pal.bg & 0xffff_ff00) | 0x2e,
        });
        v.push(Cmd::Rect { x: ax, y: ay0, w: ptw, h: ph, r: self.scale.s(10.0), color: (pal.bg & 0xffff_ff00) | 0xf2 });

        if bp_open {
            // header row (back) then the four edges
            row(v, SYS_MENU_BP_KEY, ICON_BACK, "Bar position", false, false, ay0 + mar);
            self.region(ax, ay0 + mar, ptw, row_h, SYS_MENU_BP_KEY);
            let edges: [(u32, &str, crate::shell::BarEdge, &str); 4] = [
                (SYS_EDGE_L_KEY, ICON_BACK, crate::shell::BarEdge::Left, "Left"),
                (SYS_EDGE_T_KEY, ICON_ANCHOR_UP, crate::shell::BarEdge::Top, "Top"),
                (SYS_EDGE_R_KEY, ICON_FORWARD, crate::shell::BarEdge::Right, "Right"),
                (SYS_EDGE_B_KEY, ICON_ANCHOR_DOWN, crate::shell::BarEdge::Bottom, "Bottom"),
            ];
            for (i, (key, glyph, edge, label)) in edges.iter().enumerate() {
                let ry = ay0 + mar + (i + 1) as f32 * row_h;
                row(v, *key, *glyph, label, self.bar_edge == *edge, false, ry);
                self.region(ax, ry, ptw, row_h, *key);
            }
        } else {
            row(v, SYS_MENU_BP_KEY, ICON_ARRANGE, "Bar position", false, true, ay0 + mar);
            self.region(ax, ay0 + mar, ptw, row_h, SYS_MENU_BP_KEY);
        }
    }

    /// Draw the top banner strip — ordered chip slots (workspaces / title /
    /// date / clock / tray / settings / power / etc.), separators and smart
    /// fillers. Extracted from `layout_expanded` to keep that function focused
    /// on grid + edit chrome.
    pub(crate) fn layout_banner(&mut self, v: &mut Vec<Cmd>, w: f32, lt: &Layout, cursor_px: Option<(f32, f32)>, pal: &Pal) {
        let banner_editing = self.dash_edit;
        // the declared `dashboard` surface owns the VIEWING chips (its
        // BannerRow paints them from publish_banner_cells' cells); the Rust
        // chip loop — and the viewing-only connectivity popover — stand down
        // to avoid double-painting. Edit chrome (guides, grips, tray) below
        // always stays Rust.
        let chips_via_scene = self.dash_chips_via_scene && !banner_editing;
        let strip_cells = self.banner_cells(w);
        let order = self.banner_order.clone();
        let by = lt.strip_top;
        let bh = lt.strip_h;
        // per-index zone (mirror of banner_cells' run-splitting) + the L/C/R
        // zone windows — the scissor clips each zone independently, so a
        // scrolled zone never bleeds into its neighbours
        let mut zone_of: Vec<Option<usize>> = vec![None; order.len()];
        let mut strip_zone = 0usize;
        let has_zone = order.iter().any(|t| t.as_zone().is_some());
        // whole-strip mode: the strip's natural width overflows the panel →
        // one left-aligned, horizontally scrollable row (no per-zone splitting)
        let strip_overflow = self.banner_strip_overflow();
        // per-zone scissor clipping only applies when zones exist AND the
        // strip isn't in whole-strip overflow mode
        let zoned_clip = has_zone && !strip_overflow;
        for (i, t) in order.iter().enumerate() {
            if matches!(t, BannerToken::Chip(BannerItem::DashToggle)) && !self.dash_edit {
                continue;
            }
            if let Some(z) = t.as_zone() {
                strip_zone = (z as usize).min(2);
                continue;
            }
            zone_of[i] = Some(strip_zone);
        }
        let z_margin = self.scale.s(10.0);
        let z_avail = (w - z_margin * 2.0).max(0.0);
        let z_zw = z_avail / 3.0;
        // push the right Scissor / ScissorEnd pair for a zone transition
        // (zones are contiguous runs in the order, so a single state machine
        // keeps exactly one clip active while we sweep the cells)
        let zone_push = |v: &mut Vec<Cmd>, cur: &mut Option<usize>, zc: usize| {
            if *cur != Some(zc) {
                if cur.is_some() {
                    v.push(Cmd::ScissorEnd);
                }
                *cur = Some(zc);
                v.push(Cmd::Scissor { x: z_margin + zc as f32 * z_zw, y: by, w: z_zw, h: bh });
            }
        };
        // ── edit-mode DROP GRID: a faint vertical guide at every strip
        // column (the boundary before each token, plus the tail after the
        // last one); while a token is lifted the column under the pointer is
        // highlighted as the drop target.
        let drop_slot = self.banner_drag.as_ref().and_then(|d| d.over_idx);
        let guide_col = |self_: &mut Self, v: &mut Vec<Cmd>, gx: f32, active: bool, by: f32, bh: f32| {
            v.push(Cmd::Rect {
                x: gx - 1.0,
                y: by + self_.scale.s(2.0),
                w: 2.0,
                h: bh - self_.scale.s(4.0),
                r: self_.scale.s(1.0),
                color: if active {
                    mix(ui::hover(&pal), pal.acc, 0.35)
                } else {
                    mix(ui::hover(&pal), pal.fg, 0.10)
                },
            });
        };
        if banner_editing {
            let mut gz: Option<usize> = None;
            for (i, cell) in strip_cells.iter().enumerate() {
                if let Some((cx, _)) = cell {
                    if zoned_clip {
                        zone_push(v, &mut gz, zone_of[i].unwrap_or(0));
                    }
                    guide_col(self, v, *cx, drop_slot == Some(i), by, bh);
                }
            }
            if let Some((li, last_cell)) = strip_cells.iter().enumerate().filter(|(_, c)| c.is_some()).last() {
                if zoned_clip {
                    zone_push(v, &mut gz, zone_of[li].unwrap_or(0));
                }
                if let Some((lcx, lcw)) = last_cell {
                    guide_col(self, v, lcx + lcw, drop_slot == Some(self.banner_order.len()), by, bh);
                }
            }
            if zoned_clip && gz.is_some() {
                v.push(Cmd::ScissorEnd);
            }
        }
        // tiny L / C / R section labels floating just ABOVE the strip (edit
        // mode only) so dropping a chip into a third reads clearly — only
        // when zones are actually active (not in whole-strip overflow mode)
        if banner_editing && zoned_clip && !self.banner_collapsed() {
            let margin = self.scale.s(10.0);
            let avail = (w - margin * 2.0).max(0.0);
            let zw = avail / 3.0;
            let zfs = self.scale.fs(7.0);
            let zy = by - zfs - self.scale.s(1.0);
            for (zi, zname) in ["left", "center", "right"].iter().enumerate() {
                ui::text_c(v, margin + zi as f32 * zw + zw / 2.0, zy, *zname, zfs, mix(ui::hover(&pal), pal.fg, 0.55), false);
            }
        }
        let mut active_zone: Option<usize> = None;
        // whole-strip overflow mode: one scissor clips the whole horizontally
        // scrolled row to the panel (chips pan under it, never bleed out)
        if strip_overflow {
            v.push(Cmd::Scissor { x: 0.0, y: by, w, h: bh });
        }
        // scene owns the viewing chips → the Rust chip sweep stands down (its
        // scissor is still balanced by the matching ScissorEnd below)
        for (i, item) in order.iter().enumerate() {
            if chips_via_scene {
                break;
            }
            let Some((bx, bw)) = strip_cells[i] else { continue };
            let zc = zone_of[i].unwrap_or(0);
            if zoned_clip {
                zone_push(v, &mut active_zone, zc);
                // fully scrolled out of its zone window → draw nothing (the
                // scissor would clip it anyway; skipping also drops hover +
                // hit-test regions so hidden chips can't be grabbed)
                let zx = z_margin + zc as f32 * z_zw;
                if bx + bw <= zx || bx >= zx + z_zw {
                    continue;
                }
            }
            let key = BANNER_KEY_BASE + i as u32;
            let hov = if banner_editing {
                cursor_px.map(|(px, py)| Self::in_rect(px, py, (bx, by, bw, bh))).unwrap_or(false)
            } else {
                self.hover_key == key
            };
            // the slot's own region comes FIRST so the 30+ / ✕ sub-regions
            // (pushed later) win hover hit-testing inside the slot body
            self.region(bx, by, bw, bh, key);
            let fs = self.scale.fs(11.5);
            // the token being dragged stays parked in its slot until the
            // drop — dim it there so the floating ghost reads as "lifted"
            let drag_source = banner_editing
                && self.banner_drag.as_ref().map_or(false, |d| !d.from_tray && d.from_idx == i && d.item == *item);
            let col = if drag_source { pal.sfg } else if hov { pal.acc } else { pal.fg };
            // edit mode reserves a bottom ledge of the band as a drag HANDLE —
            // chip ink floats up the band, the ledge reads as a grab-bar.
            // Raised a touch so the bar sits comfortably above the band's
            // bottom edge and is easier to grab.
            let led = if banner_editing { self.scale.s(12.0) } else { 0.0 };
            let cy = |size: f32| by + (bh - led - size) / 2.0; // ink-center every strip glyph on the band midline
            // text chips get a slight downward optical bias: a lowercase text
            // line reads "high" next to full-height icons even when both share
            // the same center (tune this to taste — +down / −up)
            let tcy = |size: f32| cy(size) + self.scale.s(TEXT_Y_BIAS);
            // EDIT MODE chip ink is an ICON — never the word label or the live
            // content — so each cell reads at a glance while staying draggable
            // ("clock" → clock glyph, "wifi" → wifi glyph, ...). DashToggle
            // keeps its live "Dash: ON/OFF" words below.
            if banner_editing {
                match item {
                    BannerToken::Chip(BannerItem::DashToggle) => {
                        let label = if self.dash_enabled { "Dash: ON" } else { "Dash: OFF" };
                        ui::text(v, bx + self.scale.s(12.0), cy(fs), label, fs, col, false);
                    }
                    BannerToken::Chip(it) => {
                        ui::text_c(v, bx + bw / 2.0, cy(self.scale.fs(14.0)), it.glyph(), self.scale.fs(14.0), col, true);
                    }
                    BannerToken::Sep(g) => {
                        // dim glyph, same row baseline as the chips
                        ui::text_c(v, bx + bw / 2.0, cy(self.scale.fs(10.0)), g, self.scale.fs(10.0), pal.sfg, false);
                    }
                    BannerToken::Filler => {
                        // invisible space — while editing, a small centered handle
                        // makes the filler grabbable / removable
                        let cx = bx + bw / 2.0;
                        let hw = self.scale.s(18.0);
                        let hh = self.scale.s(10.0);
                        v.push(Cmd::Rect {
                            x: cx - hw / 2.0,
                            y: by + (bh - hh) / 2.0,
                            w: hw,
                            h: hh,
                            r: self.scale.s(5.0),
                            color: mix(ui::hover(&pal), pal.fg, if hov { 0.30 } else { 0.10 }),
                        });
                    }
                    BannerToken::Zone(_) => {}
                }
            } else {
                match item {
                    BannerToken::Chip(BannerItem::Workspaces) => {
                        ui::text(v, bx + self.scale.s(12.0), tcy(fs), &(self.ws_active + 1).to_string(), fs, col, false);
                    }
                    BannerToken::Chip(BannerItem::Title) => {
                        let title: String = if self.title.is_empty() {
                            "Desktop".to_string()
                        } else {
                            self.title.chars().take(30).collect()
                        };
                        ui::text(v, bx + self.scale.s(12.0), tcy(fs), title, fs, col, false);
                    }
                    BannerToken::Chip(BannerItem::Date) => {
                        ui::text(v, bx + self.scale.s(12.0), tcy(fs), self.date_str("%a %d"), fs, col, false);
                    }
                    BannerToken::Chip(BannerItem::Clock) => {
                        ui::text(v, bx + self.scale.s(12.0), tcy(fs), self.clock.clone(), fs, col, false);
                    }
                    BannerToken::Chip(BannerItem::Tray) => {
                        // icons right-aligned inside the chip; each is its own
                        // 30+ sub-region so SNI activate works
                        let n = self.tray.len().min(8);
                        let mut ix = bx + bw - self.scale.s(8.0) - n as f32 * self.scale.s(20.0);
                        for ti in 0..n {
                            let ik = self.tray[ti].image_key();
                            if ik.is_empty() {
                                ui::text(v, ix + self.scale.s(2.0), cy(self.scale.fs(10.0)), ICON_TRAY_FILL, self.scale.fs(10.0), pal.fg, true);
                            } else {
                                v.push(Cmd::Image { x: ix, y: cy(self.scale.s(16.0)), w: self.scale.s(16.0), h: self.scale.s(16.0), key: ik });
                            }
                            self.region(ix, by, self.scale.s(20.0), bh, 30 + ti as u32);
                            ix += self.scale.s(20.0);
                        }
                    }
                    BannerToken::Chip(BannerItem::Settings) => {
                        // ink-centered via text_c: Nerd Font icon advances vary per
                        // glyph (a hardcoded width guess drifts off-center)
                        ui::text_c(v, bx + bw / 2.0, cy(self.scale.fs(14.0)), ICON_SETTINGS_FILL, self.scale.fs(14.0), col, true);
                    }
                    BannerToken::Chip(BannerItem::Power) => {
                        ui::text_c(v, bx + bw / 2.0, cy(self.scale.fs(14.0)), ICON_POWER_FILL, self.scale.fs(14.0), col, true);
                    }
                    BannerToken::Chip(BannerItem::AppSearch) => {
                        // app-search chip — ink-centered magnifier, opens the launcher
                        ui::text_c(v, bx + bw / 2.0, cy(self.scale.fs(14.0)), ICON_SEARCH_FILL, self.scale.fs(14.0), col, true);
                    }
                    BannerToken::Chip(BannerItem::Wifi) => {
                        // wifi chip — live glyph + SSID / state, accent while
                        // connected, dimmed while off
                        let gcol = if self.wifi_on { if self.ssid.is_empty() { pal.fg } else { pal.acc } } else { pal.sfg };
                        ui::text(v, bx + self.scale.s(12.0), cy(self.scale.fs(13.0)), ICON_WIFI_FILL, self.scale.fs(13.0), gcol, true);
                        ui::text(v, bx + self.scale.s(29.0), tcy(fs), &self.banner_wifi_text(), fs, gcol, false);
                    }
                    BannerToken::Chip(BannerItem::Bluetooth) => {
                        let gcol = if self.bt_on { pal.acc } else { pal.sfg };
                        ui::text(v, bx + self.scale.s(12.0), cy(self.scale.fs(13.0)), ICON_BLUETOOTH_FILL, self.scale.fs(13.0), gcol, true);
                        ui::text(v, bx + self.scale.s(29.0), tcy(fs), &self.banner_bt_text(), fs, gcol, false);
                    }
                    BannerToken::Chip(BannerItem::Connectivity) => {
                        // aggregate connectivity — link glyph + summary; accent
                        // on a live link, dimmed completely offline
                        let (gcol, gglyph) = if self.wifi_on && !self.ssid.is_empty() {
                            (pal.acc, ICON_LINK)
                        } else if self.wifi_on || self.bt_on {
                            (pal.fg, ICON_LINK)
                        } else {
                            (pal.sfg, ICON_LINK)
                        };
                        ui::text(v, bx + self.scale.s(12.0), cy(self.scale.fs(13.0)), gglyph, self.scale.fs(13.0), gcol, true);
                        ui::text(v, bx + self.scale.s(29.0), tcy(fs), &self.banner_conn_text(), fs, gcol, false);
                    }
                    BannerToken::Chip(BannerItem::Battery) => {
                        // live battery percent + bolt glyph; red when low &
                        // unplugged
                        let low = self.battery <= 20 && !self.ac_online;
                        let gcol = if low { RED } else { pal.fg };
                        ui::text(v, bx + self.scale.s(12.0), cy(self.scale.fs(13.0)), ui::battery_glyph(self.battery, self.ac_online), self.scale.fs(13.0), gcol, true);
                        ui::text(v, bx + self.scale.s(29.0), tcy(fs), &self.banner_battery_text(), fs, gcol, false);
                    }
                    BannerToken::Chip(BannerItem::Weather) => {
                        // outdoor temperature — glyph + temp, dim until the
                        // weather service answers
                        let (gcol, gglyph, wtxt) = match self.banner_weather_text() {
                            Some((gl, t)) => (pal.fg, gl, t),
                            None => (pal.sfg, ICON_THERMOSTAT.to_string(), "--°C".to_string()),
                        };
                        ui::text(v, bx + self.scale.s(12.0), cy(self.scale.fs(13.0)), &gglyph, self.scale.fs(13.0), gcol, true);
                        ui::text(v, bx + self.scale.s(29.0), tcy(fs), &wtxt, fs, gcol, false);
                    }
                    BannerToken::Chip(BannerItem::Cpu) => {
                        let gcol = if self.cpu >= 90 { RED } else if self.cpu >= 70 { 0xf9e2afff } else { pal.fg };
                        ui::text(v, bx + self.scale.s(12.0), cy(self.scale.fs(13.0)), ICON_SPEED_FILL, self.scale.fs(13.0), gcol, true);
                        ui::text(v, bx + self.scale.s(29.0), tcy(fs), &self.banner_cpu_text(), fs, gcol, false);
                    }
                    BannerToken::Chip(BannerItem::Ram) => {
                        let gcol = if self.mem_pct >= 90 { RED } else if self.mem_pct >= 75 { 0xf9e2afff } else { pal.fg };
                        ui::text(v, bx + self.scale.s(12.0), cy(self.scale.fs(13.0)), ICON_MEMORY, self.scale.fs(13.0), gcol, true);
                        ui::text(v, bx + self.scale.s(29.0), tcy(fs), &self.banner_ram_text(), fs, gcol, false);
                    }
                    BannerToken::Chip(BannerItem::NetSpeed) => {
                        // text-only line with ↓/↑ arrows, accent = live link
                        ui::text(v, bx + self.scale.s(12.0), tcy(fs), &self.banner_net_text(), fs, pal.acc, false);
                    }
                    BannerToken::Chip(BannerItem::Dnd) => {
                        // icon-only bell: plain = dnd off, bell-slashed + accent
                        // = do-not-disturb is on (click toggles it)
                        ui::text_c(v, bx + bw / 2.0, cy(self.scale.fs(14.0)), if self.dnd { ICON_BELL_OFF_FILL } else { ICON_BELL_FILL }, self.scale.fs(14.0), if self.dnd { pal.acc } else { pal.fg }, true);
                    }
                    BannerToken::Chip(BannerItem::Volume) => {
                        let vcol = if self.volume_muted { pal.sfg } else { pal.fg };
                        ui::text(v, bx + self.scale.s(12.0), cy(self.scale.fs(13.0)), if self.volume_muted { ICON_VOLUME_OFF_FILL } else { ICON_VOLUME_FILL }, self.scale.fs(13.0), vcol, true);
                        ui::text(v, bx + self.scale.s(29.0), tcy(fs), &self.banner_volume_text(), fs, vcol, false);
                    }
                    BannerToken::Chip(BannerItem::Brightness) => {
                        ui::text(v, bx + self.scale.s(12.0), cy(self.scale.fs(13.0)), ICON_BRIGHTNESS_FILL, self.scale.fs(13.0), pal.fg, true);
                        ui::text(v, bx + self.scale.s(29.0), tcy(fs), &self.banner_brightness_text(), fs, pal.fg, false);
                    }
                    BannerToken::Chip(BannerItem::Vpn) => {
                        let vcol = if self.ssh_vpn_lines.is_empty() { pal.sfg } else { pal.acc };
                        ui::text(v, bx + self.scale.s(12.0), cy(self.scale.fs(13.0)), ICON_LOCK_FILL, self.scale.fs(13.0), vcol, true);
                        ui::text(v, bx + self.scale.s(29.0), tcy(fs), &self.banner_vpn_text(), fs, vcol, false);
                    }
                    BannerToken::Chip(BannerItem::DashToggle) => {
                        let label = if self.dash_enabled { "Dash: ON" } else { "Dash: OFF" };
                        ui::text(v, bx + self.scale.s(12.0), cy(fs), label, fs, col, false);
                    }
                    BannerToken::Chip(BannerItem::Branding) => {
                        // brand mark — the glyph from $states2 dropped into the
                        // band as a display-only mark (no click, no hover state)
                        ui::text_cw(v, bx + bw / 2.0, cy(self.scale.fs(13.0)), &self.brand_glyph, self.scale.fs(13.0), crate::text::Ff::Brand, crate::text::Fw::Regular, pal.fg, false);
                    }
                    BannerToken::Sep(g) => {
                        // dim glyph, same row baseline as the chips
                        ui::text_c(v, bx + bw / 2.0, cy(self.scale.fs(10.0)), g, self.scale.fs(10.0), pal.sfg, false);
                    }
                    BannerToken::Filler => {
                        // invisible space — silent once not editing
                    }
                    BannerToken::Zone(_) => {}
                }
            }
            // MOVE grip under every non-filler, removable cell (edit mode): a
            // dim rounded grab-bar sitting in the reserved band bottom, a bit
            // BELOW the chip ink. It is the chip's MOVE HANDLE — the whole
            // ledge is its hit region (registered after the slot region so it
            // wins), and pressing it lifts the chip into a drag so it can be
            // re-dropped into any banner third (left / center / right). It
            // never parks. DashToggle can't be moved (keeps the dashboard
            // toggleable at its fixed anchor).
            if banner_editing
                && !matches!(item, BannerToken::Filler)
                && !matches!(item, BannerToken::Chip(BannerItem::DashToggle))
            {
                let gr = (bx, by + bh - led, bw, led);
                let ghov = cursor_px.map(|(px, py)| Self::in_rect(px, py, gr)).unwrap_or(false);
                self.region(gr.0, gr.1, gr.2, gr.3, BANNER_GRIP_KEY_BASE + i as u32);
                let gw = (self.scale.s(40.0)).min(bw - self.scale.s(6.0)).max(self.scale.s(10.0));
                let gx = bx + bw / 2.0 - gw / 2.0;
                let gy = gr.1 + self.scale.s(1.0);
                v.push(Cmd::Rect {
                    x: gx,
                    y: gy,
                    w: gw,
                    h: self.scale.s(10.0),
                    r: self.scale.s(5.0),
                    color: if ghov { mix(pal.acc, pal.fg, 0.25) } else { mix(ui::hover(&pal), pal.fg, 0.16) },
                });
            }
            // ── park-minus badge ("−" on the chip's top-right corner, EDIT
            // MODE only, hover-only): clicking it sends the chip back to the
            // parked tray. Never drawn (or hit-testable) outside edit mode —
            // in live mode the chip's own click (open its panel) is the only
            // interaction. Drawn after the chip ink and after the grip bar so
            // it reads as the top corner; its region is pushed last, so it
            // wins the press over the slot body.
            let removable = !matches!(item, BannerToken::Filler)
                && !matches!(item, BannerToken::Chip(BannerItem::DashToggle));
            if banner_editing && removable {
                let pk = BANNER_PARK_KEY_BASE + i as u32;
                let bd = self.scale.s(15.0);
                let bx0 = bx + bw - bd - self.scale.s(3.0);
                let by0 = by + self.scale.s(2.0);
                // hovered when the cursor sits in the cell (edit mode means
                // the cursor drives the strip, not hover_key)
                let badge_hov = cursor_px.map(|(px, py)| Self::in_rect(px, py, (bx0, by0, bd, bd))).unwrap_or(false);
                if hov {
                    v.push(Cmd::Rect {
                        x: bx0,
                        y: by0,
                        w: bd,
                        h: bd,
                        r: bd / 2.0,
                        color: if badge_hov { mix(ui::hover(&pal), pal.fg, 0.34) } else { mix(ui::hover(&pal), pal.fg, 0.13) },
                    });
                    // a real horizontal minus bar (drawn, not a font glyph —
                    // no tofu risk at this tiny size)
                    let mw = self.scale.s(8.0);
                    let mh = self.scale.s(2.4);
                    v.push(Cmd::Rect {
                        x: bx0 + (bd - mw) / 2.0,
                        y: by0 + (bd - mh) / 2.0,
                        w: mw,
                        h: mh,
                        r: mh / 2.0,
                        color: pal.fg,
                    });
                    self.region(bx0 - self.scale.s(3.0), by0 - self.scale.s(3.0), bd + self.scale.s(6.0), bd + self.scale.s(6.0), pk);
                }
            }
        }
        if strip_overflow {
            v.push(Cmd::ScissorEnd);
        } else if zoned_clip && active_zone.is_some() {
            v.push(Cmd::ScissorEnd);
        }
        // connectivity aggregate popover: the "net" chip expands a small
        // jump menu beneath the strip with Wi-Fi / Bluetooth shortcut rows
        // (each chevron opens its full menu). Drawn over the board, and over
        // the strip cells, so it closes by pressing anywhere else. Viewing
        // only — skipped when the scene owns the chips (it reads the Rust
        // chip geometry, which stood down).
        if self.connectivity_open && !self.dash_edit && !chips_via_scene {
            if let Some(idx) = self.banner_order.iter().position(|t| matches!(t, BannerToken::Chip(BannerItem::Connectivity))) {
                if let Some((cbx, cbw, _, _)) = self.banner_cell_px(idx, w) {
                    let ptw = self.scale.s(150.0);
                    let px0 = (cbx + cbw / 2.0 - ptw / 2.0)
                        .clamp(self.scale.s(8.0), (w - ptw - self.scale.s(8.0)).max(self.scale.s(8.0)));
                    let row_h = self.scale.s(30.0);
                    let py0 = by + bh + self.scale.s(8.0);
                    let ph = row_h * 2.0 + self.scale.s(6.0);
                    let gfs = self.scale.fs(13.0);
                    let tfs = self.scale.fs(11.0);
                    // soft shadow ring + opaque body
                    v.push(Cmd::Rect {
                        x: px0 - self.scale.s(3.0),
                        y: py0 - self.scale.s(3.0),
                        w: ptw + self.scale.s(6.0),
                        h: ph + self.scale.s(6.0),
                        r: self.scale.s(12.0),
                        color: (pal.bg & 0xffffff00) | 0x2e,
                    });
                    v.push(Cmd::Rect { x: px0, y: py0, w: ptw, h: ph, r: self.scale.s(10.0), color: (pal.bg & 0xffffff00) | 0xf2 });
                    let lw = |s: &str| s.chars().count() as f32 * 0.58 * tfs;
                    let vy_off = self.scale.s(1.0);
                    let vy = move |ry: f32| ry + (row_h - tfs) / 2.0 + vy_off;
                    // ── Wi-Fi row
                    let wy = py0 + self.scale.s(3.0);
                    let whov = cursor_px.map(|(px, py)| Self::in_rect(px, py, (px0, wy, ptw, row_h))).unwrap_or(false);
                    if whov {
                        v.push(Cmd::Rect { x: px0 + self.scale.s(3.0), y: wy, w: ptw - self.scale.s(6.0), h: row_h, r: self.scale.s(8.0), color: ui::hover_hl(&pal) });
                    }
                    let wcol = if self.wifi_on { pal.acc } else { pal.sfg };
                    ui::text(v, px0 + self.scale.s(12.0), vy(wy) - self.scale.s(1.0), ICON_WIFI_FILL, gfs, wcol, true);
                    ui::text(v, px0 + self.scale.s(10.0), vy(wy), "Wi-Fi", tfs, pal.fg, true);
                    ui::text(v, px0 + self.scale.s(10.0) + lw("Wi-Fi"), vy(wy), &self.banner_wifi_text(), tfs, wcol, false);
                    ui::text_r(v, px0 + ptw - self.scale.s(12.0), vy(wy), ICON_ARROW_FORWARD, tfs, pal.sfg, true);
                    self.region(px0, wy, ptw, row_h, CONN_WIFI_KEY);
                    // ── Bluetooth row
                    let by2 = wy + row_h;
                    let bhov = cursor_px.map(|(px, py)| Self::in_rect(px, py, (px0, by2, ptw, row_h))).unwrap_or(false);
                    if bhov {
                        v.push(Cmd::Rect { x: px0 + self.scale.s(3.0), y: by2, w: ptw - self.scale.s(6.0), h: row_h, r: self.scale.s(8.0), color: ui::hover_hl(&pal) });
                    }
                    let bcol = if self.bt_on { pal.acc } else { pal.sfg };
                    ui::text(v, px0 + self.scale.s(12.0), vy(by2) - self.scale.s(1.0), ICON_BLUETOOTH_FILL, gfs, bcol, true);
                    ui::text(v, px0 + self.scale.s(10.0), vy(by2), "Bluetooth", tfs, pal.fg, true);
                    ui::text(v, px0 + self.scale.s(10.0) + lw("Bluetooth"), vy(by2), &self.banner_bt_text(), tfs, bcol, false);
                    ui::text_r(v, px0 + ptw - self.scale.s(12.0), vy(by2), ICON_ARROW_FORWARD, tfs, pal.sfg, true);
                    self.region(px0, by2, ptw, row_h, CONN_BT_KEY);
                }
            }
        }
        // edit mode: a horizontal separator + breathing room directly below
        // the banner band — it visually splits the strip from the parked
        // tray / editor rows stacked beneath it
        if banner_editing {
            let sep_y = lt.strip_top + lt.strip_h + self.scale.s(5.0);
            v.push(Cmd::Rect {
                x: lt.pad,
                y: sep_y,
                w: w - lt.pad * 2.0,
                h: self.scale.s(1.0),
                r: self.scale.s(0.5),
                color: mix(ui::hover(&pal), pal.fg, 0.18),
            });
        }
        // reorder drag: the lifted slot floats under the cursor
        if banner_editing {
            if let Some(d) = self.banner_drag.as_ref() {
                let dw = if matches!(d.item, BannerToken::Filler) {
                    self.scale.s(64.0)
                } else {
                    self.banner_token_w(&d.item)
                };
                let dx = (d.x - dw / 2.0).clamp(4.0, (w - dw - 4.0).max(4.0));
                let dy = lt.strip_top - self.scale.s(10.0);
                v.push(Cmd::Rect {
                    x: dx,
                    y: dy,
                    w: dw,
                    h: self.scale.s(30.0),
                    r: self.scale.s(13.0),
                    color: mix(ui::hover(&pal), pal.acc, 0.18),
                });
                ui::text_c(v, dx + dw / 2.0, dy + self.scale.s(5.0), d.item.label(), self.scale.fs(11.5), pal.fg, false);
            }
        }
    }

    /// Small edit-mode button chip (grid rows/cols "+"/"-" toolbar).
    pub(crate) fn draw_edit_btn(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, bw: f32, bh: f32, key: u32, sym: &str, cursor: Option<(f32, f32)>, pal: &Pal, enabled: bool) {
        let hov = enabled && cursor.map(|(px, py)| Self::in_rect(px, py, (x, y, bw, bh))).unwrap_or(false);
        v.push(Cmd::Rect {
            x,
            y,
            w: bw,
            h: bh,
            r: self.scale.s(6.0),
            // disabled: flat, no hover, no region — reads as inert
            color: if !enabled {
                mix(ui::hover(pal), pal.fg, 0.03)
            } else if hov {
                ui::hover_hl(pal)
            } else {
                mix(ui::hover(pal), pal.fg, 0.10)
            },
        });
        ui::text_c(
            v,
            x + bw / 2.0,
            y + self.scale.s(3.5),
            sym,
            self.scale.fs(9.5),
            if enabled { pal.fg } else { mix(ui::hover(pal), ui::fg3(pal), 0.6) },
            false,
        );
        if enabled {
            self.region(x, y, bw, bh, key);
        }
    }

    /// Labeled edit-mode button (strip-editor row). `accent` tints the chip
    /// with the palette accent so the primary action stands out.
    pub(crate) fn draw_edit_lbl_btn(
        &mut self,
        v: &mut Vec<Cmd>,
        x: f32,
        y: f32,
        bw: f32,
        bh: f32,
        key: u32,
        label: &str,
        cursor: Option<(f32, f32)>,
        pal: &Pal,
        accent: bool,
    ) {
        let hov = cursor.map(|(px, py)| Self::in_rect(px, py, (x, y, bw, bh))).unwrap_or(false);
        let bg = if accent {
            if hov { mix(ui::hover(&pal), pal.acc, 0.25) } else { (pal.acc & 0xFFFF_FF00) | 0x28 }
        } else if hov {
            ui::hover_hl(pal)
        } else {
            mix(ui::hover(pal), pal.fg, 0.10)
        };
        v.push(Cmd::Rect { x, y, w: bw, h: bh, r: self.scale.s(6.0), color: bg });
        let fs = self.scale.fs(9.5);
        let ty = y + (bh - fs) / 2.0;
        ui::text_c(v, x + bw / 2.0, ty, label, fs, if accent { pal.acc } else { pal.fg }, false);
        self.region(x, y, bw, bh, key);
    }

    /// Edit-mode dual-glyph direction toggle (the "stack" scroll button):
    /// both glyphs render side by side inside one chip — the active direction
    /// is accent-highlighted, the idle one dimmed. Always registers a hit
    /// region (a click flips the direction).
    fn draw_edit_toggle_btn(&mut self, v: &mut Vec<Cmd>, x: f32, y: f32, bw: f32, bh: f32, key: u32, sym_a: &str, sym_b: &str, a_active: bool, cursor: Option<(f32, f32)>, pal: &Pal) {
        let hov = cursor.map(|(px, py)| Self::in_rect(px, py, (x, y, bw, bh))).unwrap_or(false);
        let bg = if hov { ui::hover_hl(pal) } else { mix(ui::hover(pal), pal.fg, 0.10) };
        v.push(Cmd::Rect { x, y, w: bw, h: bh, r: self.scale.s(6.0), color: bg });
        let dim = mix(ui::hover(pal), ui::fg3(pal), 0.6);
        let half = bw / 2.0;
        let fs = self.scale.fs(9.5);
        ui::text_c(v, x + half / 2.0, y + self.scale.s(3.5), sym_a, fs, if a_active { pal.acc } else { dim }, false);
        ui::text_c(v, x + half + half / 2.0, y + self.scale.s(3.5), sym_b, fs, if a_active { dim } else { pal.acc }, false);
        self.region(x, y, bw, bh, key);
    }

    /// One edit-mode UI-controls stepper cell: label on top, then a
    /// "− <readout> +" row (the old pad / win / card sliders, now buttons).
    /// `enabled == false` grays the whole cell and drops the hit regions
    /// (card radius while it is synced to the window radius).
    #[allow(clippy::too_many_arguments)]
    fn draw_ui_ctrl_stepper(
        &mut self,
        v: &mut Vec<Cmd>,
        x: f32,
        w: f32,
        label_y: f32,
        ctrl_y: f32,
        label: &str,
        readout: &str,
        minus_key: u32,
        plus_key: u32,
        enabled: bool,
        cursor: Option<(f32, f32)>,
        pal: &Pal,
    ) {
        let fs = self.scale.fs(9.0);
        let dim = || mix(ui::hover(pal), ui::fg3(pal), 0.6);
        ui::text(v, x, label_y, label, fs, if enabled { pal.fg } else { dim() }, false);
        let bw = self.scale.s(20.0);
        let bh = self.scale.s(18.0);
        let by = ctrl_y + (bw - bh) / 2.0;
        let ro_w = readout.chars().count() as f32 * 0.56 * fs;
        self.draw_edit_btn(v, x, by, bw, bh, minus_key, "-", cursor, pal, enabled);
        ui::text(v, x + (w - ro_w) / 2.0, ctrl_y + self.scale.s(4.0), readout, fs, if enabled { pal.fg } else { dim() }, false);
        self.draw_edit_btn(v, x + w - bw, by, bw, bh, plus_key, "+", cursor, pal, enabled);
    }

        pub(crate) fn card_name(card: DashCard) -> &'static str {
        match card {
            DashCard::System => "System",
            DashCard::Weather => "Weather",
            DashCard::WeatherV2 => "Weather v2",
            DashCard::Media => "Media",
            DashCard::Network => "Network",
            DashCard::CpuGpu => "CPU/GPU",
            DashCard::Cpu => "CPU",
            DashCard::ClipImg => "Clipboard images",
            DashCard::Gauges => "Gauges",
            DashCard::Todo => "To-Do",
            DashCard::Toggles => "Toggles",
            DashCard::Sliders => "Sliders",
            DashCard::Disk => "Disk",
            DashCard::ProcMon => "Processes",
            DashCard::Mem => "Memory",
            DashCard::ActiveWin => "Active Win",
            DashCard::Clipboard => "Clipboard",
            DashCard::Notes => "Notes",
            DashCard::PkgUpdates => "Updates",
            DashCard::SshVpn => "SSH/VPN",
            DashCard::Docker => "Docker",
            DashCard::KbLayout => "Keyboard",
            DashCard::Wallpaper => "Wallpapers",
            DashCard::News => "News",
            DashCard::Calendar => "Calendar",
            DashCard::Accent => "Accent",
            DashCard::CustomAcc => "Custom Accents",
            DashCard::Viz => "Visualizer",
            DashCard::Sensors => "Sensors",
            DashCard::Mirror => "Camera",
            DashCard::PowerV => "Power V",
            DashCard::PowerH => "Power H",
            DashCard::DiskIo => "Disk I/O",
            DashCard::Lyrics => "Lyrics",
            DashCard::Wifi => "Wi-Fi",
            DashCard::Bluetooth => "Bluetooth",
            DashCard::SpeedTest => "Speed test",
            DashCard::Recent => "Recent files",
            DashCard::Currency => "Currency",
            DashCard::Quote => "Quote",
            DashCard::Ticker => "Prices",
            DashCard::Gpu => "GPU",
            DashCard::Thermal => "Thermals",
            DashCard::Clock => "Clock",
            DashCard::BatteryV => "Battery V",
            DashCard::BatteryH => "Battery H",
            DashCard::Eq => "Equalizer",
            DashCard::AppShortcut => "Apps",
            DashCard::WorldMap => "World",
            DashCard::Pomodoro => "Focus",
            DashCard::Fans => "Fans",
            DashCard::Workspaces => "Workspaces",
            DashCard::WorldClock => "World clock",
            DashCard::Water => "Water",
            DashCard::Moon => "Moon",
            DashCard::Compositor => "Effects",
            DashCard::Countdown => "Countdown",
            DashCard::Alarms => "Alarms",
            DashCard::Snippets => "Snippets",
            DashCard::Expenses => "Expenses",
            DashCard::EyeRest => "Eye rest",
            DashCard::MicMeter => "Mic meter",
            DashCard::AudioDevice => "Audio devices",
            DashCard::AudioRec => "Audio recorder",
            DashCard::ConnInfo => "Network info",
            DashCard::Latency => "Latency",
            DashCard::PowerDraw => "Power draw",
            DashCard::SmartHealth => "Disk health",
            DashCard::SystemdUnits => "Services",
            DashCard::JournalTail => "System log",
            DashCard::Branding => "Branding",
        }
    }

    // ---------------------------------------------------- metro card drawers
    // Each drawer paints one card at an arbitrary pixel rect (grid spans),
    // registering its own hit regions. They degrade gracefully when squeezed.
}

#[cfg(test)]
mod card_header_toggle_tests {
    use super::*;

    /// A dashboard shell with the Bluetooth card on the board, isolated from
    /// the live per-channel config (save_config writes there).
    fn bt_shell(tag: &str) -> Shell {
        let cfg: Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        let mut s = Shell::new(cfg);
        s.per_state_config =
            std::env::temp_dir().join(format!("zen-test-card-toggles-{tag}.toml"));
        s.mode = crate::shell::Mode::Expanded;
        s.dash_enabled = true;
        s.dash_layout =
            vec![CardLayout { card: DashCard::Bluetooth, x: 0, y: 0, w: 6, h: 4 }];
        s.refresh_pack();
        s
    }

    /// Both toggles render in the strip-editor row (edit mode, dashboard ON)
    /// and register clickable regions under their keys.
    #[test]
    fn header_toggles_draw_and_register_in_edit_mode() {
        let mut s = bt_shell("titles-draw");
        s.dash_edit = true;
        s.layout(1200.0, 800.0);
        let keys: Vec<u32> = s.hover_regions.iter().map(|r| r.4).collect();
        assert!(keys.contains(&CARD_TITLE_TOGGLE_KEY), "titles toggle must register");
        assert!(keys.contains(&CARD_GLYPH_TOGGLE_KEY), "glyphs toggle must register");
    }

    /// Toggles only exist while the dashboard is enabled — with the board
    /// off there is nothing to label.
    #[test]
    fn header_toggles_hidden_when_dashboard_off() {
        let mut s = bt_shell("dash-off");
        s.dash_enabled = false;
        s.dash_edit = true;
        s.layout(1200.0, 800.0);
        let keys: Vec<u32> = s.hover_regions.iter().map(|r| r.4).collect();
        assert!(!keys.contains(&CARD_TITLE_TOGGLE_KEY), "no titles toggle without the board");
        assert!(!keys.contains(&CARD_GLYPH_TOGGLE_KEY), "no glyphs toggle without the board");
    }

    /// Clicking "Card titles" flips the flag, persists it to the channel
    /// config, and the next frame drops every card's header title TEXT.
    #[test]
    fn titles_toggle_flips_persists_and_hides_titles() {
        let mut s = bt_shell("titles");
        assert!(s.card_show_title, "titles default ON");
        s.dash_edit = true;
        s.layout(1200.0, 800.0);
        let (rx, ry, _, _, _) = s.hover_regions.iter().find(|r| r.4 == CARD_TITLE_TOGGLE_KEY).copied().expect("toggle registered");
        s.edit_press(rx + 1.0, ry + 1.0);
        assert!(!s.card_show_title, "click flips the toggle");
        let saved = std::fs::read_to_string(&s.per_state_config).unwrap_or_default();
        assert!(saved.contains("card_show_title = false"), "persisted to the channel config");
        // the drawer must obey: no card title TEXT anywhere in the scene
        let v = s.layout(1200.0, 800.0);
        let has_bt_title = v.iter().any(
            |c| matches!(c, Cmd::Text { text, .. } if text == "Bluetooth"),
        );
        assert!(!has_bt_title, "header title hidden after toggling off");
        // and back on
        let (rx, ry, _, _, _) = s.hover_regions.iter().find(|r| r.4 == CARD_TITLE_TOGGLE_KEY).copied().unwrap();
        s.edit_press(rx + 1.0, ry + 1.0);
        assert!(s.card_show_title, "second click restores");
        let v = s.layout(1200.0, 800.0);
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "Bluetooth")),
            "header title drawn again"
        );
        let _ = std::fs::remove_file(&s.per_state_config);
    }

    /// Same flow for the glyph toggle: the header ICON disappears while the
    /// title TEXT stays.
    #[test]
    fn glyphs_toggle_hides_icon_keeps_title() {
        let mut s = bt_shell("glyphs");
        assert!(s.card_show_glyph, "glyphs default ON");
        s.dash_edit = true;
        s.layout(1200.0, 800.0);
        let (rx, ry, _, _, _) = s.hover_regions.iter().find(|r| r.4 == CARD_GLYPH_TOGGLE_KEY).copied().unwrap();
        s.edit_press(rx + 1.0, ry + 1.0);
        assert!(!s.card_show_glyph, "click flips the toggle");
        let saved = std::fs::read_to_string(&s.per_state_config).unwrap_or_default();
        assert!(saved.contains("card_show_glyph = false"), "persisted");
        let v = s.layout(1200.0, 800.0);
        let glyph = crate::icons::ICON_BLUETOOTH;
        let has_icon = v.iter().any(
            |c| matches!(c, Cmd::Text { text, icon, .. } if *icon && text == glyph),
        );
        assert!(!has_icon, "header icon hidden after toggling off");
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "Bluetooth")),
            "title text STAYS"
        );
        let _ = std::fs::remove_file(&s.per_state_config);
    }
}
