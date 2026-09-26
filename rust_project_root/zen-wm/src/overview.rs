//! Workspace overview: a grid of scaled window thumbnails that grabs all keyboard
//! input while active. Arrows (with or without the mod key) navigate; Return
//! selects; Escape cancels.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverviewDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug)]
pub struct OverviewState {
    pub active: bool,
    /// Index into `workspaces.list` currently highlighted.
    pub selected: usize,
    /// Workspace to return to if the overview is cancelled.
    pub previous: usize,
}

impl OverviewState {
    pub fn new() -> Self {
        Self {
            active: false,
            selected: 0,
            previous: 0,
        }
    }

    pub fn toggle(&mut self, active_ws: usize) {
        if self.active {
            self.exit();
        } else {
            self.enter(active_ws);
        }
    }

    pub fn enter(&mut self, active_ws: usize) {
        self.active = true;
        self.previous = active_ws;
        self.selected = active_ws;
    }

    pub fn exit(&mut self) {
        self.active = false;
    }

    pub fn selected_workspace(&self) -> usize {
        self.selected
    }

    /// Move the selection in the grid (wrapping at the edges).
    pub fn move_selection(&mut self, direction: OverviewDirection, columns: u32, rows: u32, count: usize) {
        if count == 0 {
            return;
        }
        let cols = columns.max(1) as usize;
        let rows = rows.max(1) as usize;
        let row = self.selected / cols;
        let col = self.selected % cols;

        let (new_row, new_col) = match direction {
            OverviewDirection::Left => (row, if col == 0 { cols - 1 } else { col - 1 }),
            OverviewDirection::Right => (row, if col + 1 >= cols { 0 } else { col + 1 }),
            OverviewDirection::Up => (if row == 0 { rows - 1 } else { row - 1 }, col),
            OverviewDirection::Down => (if row + 1 >= rows { 0 } else { row + 1 }, col),
        };

        let mut idx = (new_row * cols + new_col) % count;
        // Clamp to a valid workspace index.
        if idx >= count {
            idx = count - 1;
        }
        self.selected = idx;
    }

    pub fn move_relative(&mut self, delta: isize, count: usize) {
        if count == 0 {
            return;
        }
        self.selected = (self.selected as isize + delta).rem_euclid(count as isize) as usize;
    }
}

impl Default for OverviewState {
    fn default() -> Self {
        Self::new()
    }
}
