use smithay::{
    desktop::{space::SpaceElement, Space},
    output::Output,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point, Rectangle, Size},
};

use crate::element::WindowElement;

/// Which side the master window occupies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitSide {
    #[default]
    Left,
    Right,
}

/// A single workspace: an independent stacking [`Space`] of windows.
pub struct Workspace {
    pub id: usize,
    pub space: Space<WindowElement>,
    pub master_side: SplitSide,
}

impl Workspace {
    fn new(id: usize) -> Self {
        Self {
            id,
            space: Space::default(),
            master_side: SplitSide::Left,
        }
    }
}

/// All workspaces + active-workspace bookkeeping.
pub struct Workspaces {
    pub list: Vec<Workspace>,
    pub active: usize,
    pub columns: u32,
    pub rows: u32,
}

impl Workspaces {
    pub fn new(count: usize, columns: u32, rows: u32) -> Self {
        Self {
            list: (1..=count).map(Workspace::new).collect(),
            active: 0,
            columns,
            rows,
        }
    }

    pub fn count(&self) -> usize {
        self.list.len()
    }

    /// Grow or shrink the workspace list to `new_count` (min 1).
    ///
    /// Growing appends empty workspaces. Shrinking is **refused** (returns `false`)
    /// if any workspace beyond the new count still contains windows, so no window
    /// is ever silently dropped. The active index is clamped on shrink.
    pub fn resize(&mut self, new_count: usize) -> bool {
        let n = new_count.max(1);
        if n == self.list.len() {
            return true;
        }
        if n < self.list.len() {
            if self.list[n..].iter().any(|ws| ws.space.elements().count() > 0) {
                return false;
            }
            self.list.truncate(n);
            if self.active >= n {
                self.active = n - 1;
            }
            return true;
        }
        while self.list.len() < n {
            self.list.push(Workspace::new(self.list.len() + 1));
        }
        true
    }

    pub fn active_workspace(&self) -> &Workspace {
        &self.list[self.active]
    }

    pub fn active_workspace_mut(&mut self) -> &mut Workspace {
        &mut self.list[self.active]
    }

    /// Map the output into every workspace so window geometry is consistent;
    /// only the active workspace is ever rendered.
    pub fn map_output(&mut self, output: &Output) {
        for ws in &mut self.list {
            ws.space.map_output(output, (0, 0));
        }
    }

    pub fn window_for_surface(&self, surface: &WlSurface) -> Option<WindowElement> {
        self.list.iter().find_map(|ws| {
            ws.space
                .elements()
                .find(|w| w.wl_surface().map(|s| &*s == surface).unwrap_or(false))
                .cloned()
        })
    }

    pub fn workspace_for_window(&self, window: &WindowElement) -> Option<usize> {
        self.list
            .iter()
            .position(|ws| ws.space.elements().any(|w| w == window))
    }

    pub fn active_window(&self) -> Option<WindowElement> {
        self.active_workspace().space.elements().last().cloned()
    }

    pub fn num_windows(&self, idx: usize) -> usize {
        self.list[idx].space.elements().count()
    }

    pub fn total_windows(&self) -> usize {
        self.list.iter().map(|ws| ws.space.elements().count()).sum()
    }

    /// Drop dead windows from all spaces; returns true if anything was removed.
    pub fn refresh(&mut self) -> bool {
        let mut removed = false;
        for ws in &mut self.list {
            let before = ws.space.elements().count();
            ws.space.refresh();
            if ws.space.elements().count() != before {
                removed = true;
            }
        }
        removed
    }

    pub fn switch_to(&mut self, idx: usize) {
        if idx >= self.list.len() || idx == self.active {
            return;
        }
        // Deactivate everything in the old workspace.
        for el in self.list[self.active].space.elements() {
            el.set_activate(false);
        }
        self.active = idx;
        // Activate the frontmost window of the new workspace.
        if let Some(el) = self.list[idx].space.elements().last() {
            el.set_activate(true);
        }
    }

    /// Next/prev workspace that actually contains windows; stays put if none.
    pub fn switch_relative(&mut self, delta: isize) -> bool {
        let n = self.list.len();
        if n == 0 {
            return false;
        }
        let mut candidate = (self.active as isize + delta).rem_euclid(n as isize) as usize;
        while candidate != self.active && self.list[candidate].space.elements().count() == 0 {
            candidate = (candidate as isize + delta).rem_euclid(n as isize) as usize;
        }
        if candidate != self.active && self.list[candidate].space.elements().count() > 0 {
            self.switch_to(candidate);
            true
        } else {
            false
        }
    }

    /// Tile the given workspace, then map each window.
    ///
    /// `layout` is the config's `layout` field: `"scrolling"` (default) arranges
    /// windows as a full-width vertical column (niri-flavored); `"master"` uses the
    /// dwm-style master/stack split.
    pub fn layout_workspace(&mut self, idx: usize, output: &Output, gap: i32, layout: &str) {
        let Some(output_geo) = self.list[idx].space.output_geometry(output) else {
            return;
        };
        let gap = gap.max(0);
        let inner = Rectangle::new(
            Point::from((output_geo.loc.x + gap, output_geo.loc.y + gap)),
            Size::from((
                (output_geo.size.w - 2 * gap).max(0),
                (output_geo.size.h - 2 * gap).max(0),
            )),
        );

        let windows: Vec<WindowElement> = self.list[idx].space.elements().cloned().collect();
        let n = windows.len();
        if n == 0 {
            return;
        }

        if n == 1 {
            self.map_and_configure(&windows[0], inner);
            return;
        }

        if layout != "master" {
            // Scrolling layout: full-width vertical column, equal heights.
            let total_gap = (n as i32 - 1) * gap;
            let base_h = ((inner.size.h - total_gap) / n as i32).max(1);
            let rem = (inner.size.h - total_gap).max(0) % n as i32;
            for (i, w) in windows.iter().enumerate() {
                let h = base_h + if (i as i32) < rem { 1 } else { 0 };
                let rect = Rectangle::new(
                    Point::from((inner.loc.x, inner.loc.y + i as i32 * (base_h + gap))),
                    Size::from((inner.size.w, h)),
                );
                self.map_and_configure(w, rect);
            }
            return;
        }

        // Master layout: window 0 = master on `master_side` half, rest stack vertically.
        let master = &windows[0];
        let stack = &windows[1..];
        let master_side = self.list[idx].master_side;

        let master_w = ((inner.size.w - gap) / 2).max(1);
        let master_rect = match master_side {
            SplitSide::Left => Rectangle::new(inner.loc, Size::from((master_w, inner.size.h))),
            SplitSide::Right => Rectangle::new(
                Point::from((inner.loc.x + inner.size.w - master_w, inner.loc.y)),
                Size::from((master_w, inner.size.h)),
            ),
        };
        let stack_x = match master_side {
            SplitSide::Left => inner.loc.x + master_w + gap,
            SplitSide::Right => inner.loc.x,
        };
        let stack_w = (inner.size.w - master_w - gap).max(1);

        self.map_and_configure(master, master_rect);

        let s = stack.len();
        if s > 0 {
            let total = (stack_w.max(0), inner.size.h.max(0));
            let space_h = (total.1 - (s as i32 - 1) * gap).max(0) / s as i32;
            let rem = (total.1 - (s as i32 - 1) * gap).max(0) % s as i32;
            for (i, w) in stack.iter().enumerate() {
                let h = space_h + if (i as i32) < rem { 1 } else { 0 };
                let rect = Rectangle::new(
                    Point::from((stack_x, inner.loc.y + i as i32 * (space_h + gap))),
                    Size::from((total.0, h)),
                );
                self.map_and_configure(w, rect);
            }
        }
    }

    fn map_and_configure(&mut self, window: &WindowElement, rect: Rectangle<i32, Logical>) {
        let idx = self.active;
        self.list[idx].space.map_element(window.clone(), rect.loc, false);
        window.configure(rect);
    }
}
