//! Pre-computed dashboard geometry — computed once per frame, read everywhere.
//!
//! Every geometry value the dashboard needs (`grid_top`, `banner_strip_top`,
//! `tray_gap`, etc.) is computed here into a flat struct so the drawing code
//! never recalculates positions mid-render. This is the single source of
//! truth for dashboard spatial layout.

use super::*;

/// All dashboard geometry, computed once at the start of `layout_expanded`.
/// Read-only after construction — no method calls, just field access.
pub(crate) struct Layout {
    // ── surface ──
    // w/h are the surface size the layout was computed for; asserted by tests.
    #[cfg_attr(not(test), allow(dead_code))]
    pub w: f32,
    #[cfg_attr(not(test), allow(dead_code))]
    pub h: f32,
    pub editing: bool,

    // ── grid cell geometry ──
    pub cell_w: f32,
    pub cell_h: f32,
    pub pad: f32,
    pub gap: f32,
    pub card_r: f32,

    // ── canvas ──
    pub canvas_w: f32,
    pub canvas_h: f32,
    pub scroll_x: f32,
    pub scroll_y: f32,

    // ── banner strip ──
    pub strip_top: f32,
    pub strip_h: f32,

    // ── edit-mode chrome (only valid when `editing` is true) ──
    pub tray_top: f32,        // banner-chips parking tray top
    pub banner_tray_h: f32,   // banner-chips parking tray content height
    pub banner_ctrl_top: f32, // strip-editor row top
    pub banner_ctrl_h: f32,   // strip-editor row height
    pub ui_ctrl_top: f32,     // UI controls row top
    pub ui_ctrl_h: f32,       // UI controls row height
    pub tray_gap: f32,        // gap between stacked tray rows

    // ── composite ──
    /// Y where the card grid starts — below all chrome in edit mode,
    /// at `dash_top()` otherwise.
    pub grid_top: f32,
    /// Bottom edge of the z-order mask (covers everything above the grid).
    pub mask_bottom: f32,
    /// Opaque background color (bg with full alpha).
    pub opaque: u32,
    /// Panel corner radius (for floating/top-edge shaping).
    pub cr: f32,
}

impl Layout {
    /// Draw an opaque background strip for a z-order layer. This ensures
    /// scrolled cards can never bleed through a chrome panel. Call once per
    /// layer before drawing its content.
    ///
    /// # Z-order (back to front)
    /// 1. **Backdrop** (y=0 → `grid_top`) — opaque surface fill
    /// 2. **Cards** — scrollable grid, drawn in content space
    /// 3. **Chrome mask** (y=0 → `mask_bottom`) — re-covers any cards that
    ///    scrolled above the chrome zone
    /// 4. **Chrome panels** — each draws its own opaque slab + rounded rect
    /// 5. **Banner strip** — drawn last, slides under by z-position
    pub fn draw_chrome_mask(&self, v: &mut Vec<Cmd>, w: f32) {
        if !self.editing {
            return;
        }
        let bottom = self.mask_bottom;
        if bottom <= 0.0 {
            return;
        }
        let cap_h = self.cr.min(bottom);
        let body_top = cap_h;
        if body_top < bottom {
            v.push(Cmd::Rect {
                x: 0.0,
                y: body_top,
                w,
                h: bottom - body_top,
                r: 0.0,
                color: self.opaque,
            });
        }
        if cap_h > 0.0 {
            v.push(Cmd::RectConcave {
                x: 0.0,
                y: 0.0,
                w,
                h: cap_h,
                r_tl: self.cr,
                r_tr: self.cr,
                r_br: 0.0,
                r_bl: 0.0,
                color: self.opaque,
            });
        }
    }

    /// Compute all dashboard geometry from the current Shell state.
    pub fn compute(shell: &Shell, w: f32, h: f32) -> Self {
        let editing = shell.dash_edit;

        let cell_w = shell.dash_cell_base_w(shell.cfg.expanded_w());
        let cell_h = shell.dash_cell_h(shell.cfg.expanded_w());
        let pad = shell.dash_pad();
        let gap = shell.dash_gap();
        let card_r = shell.card_r();
        let panel_r = shell.panel_r();

        let canvas_w = shell.dash_canvas_w();
        let canvas_h = shell.dash_canvas_h();
        let scroll_x = shell.dash_scroll_x;
        let scroll_y = shell.dash_scroll_y;

        let strip_top = shell.banner_strip_top();
        let strip_h = shell.banner_strip_h();

        // edit-mode chrome geometry
        let tray_top = shell.tray_at_top();
        let banner_tray_h = shell.banner_tray_h();
        let banner_ctrl_top = shell.banner_ctrl_top();
        let banner_ctrl_h = shell.banner_ctrl_h();
        let ui_ctrl_h = shell.ui_ctrl_h();
        let tray_gap = shell.tray_gap();

        let ui_ctrl_top = shell.ui_ctrl_top();

        let grid_top = shell.grid_top();

        let mask_bottom = if editing && shell.dash_enabled {
            grid_top.max(ui_ctrl_top + ui_ctrl_h)
        } else if editing {
            grid_top
        } else {
            0.0
        };

        let bg = shell.sv_bg;
        let opaque = (bg & 0xffffff00) | 0xff;

        let cr = if shell.bar_floating || shell.bar_edge == crate::shell::BarEdge::Bottom {
            panel_r
        } else {
            0.0
        };

        Self {
            w,
            h,
            editing,
            cell_w,
            cell_h,
            pad,
            gap,
            card_r,
            canvas_w,
            canvas_h,
            scroll_x,
            scroll_y,
            strip_top,
            strip_h,
            tray_top,
            banner_tray_h,
            banner_ctrl_top,
            banner_ctrl_h,
            ui_ctrl_top,
            ui_ctrl_h,
            tray_gap,
            grid_top,
            mask_bottom,
            opaque,
            cr,
        }
    }
}

#[cfg(test)]
mod tests;
