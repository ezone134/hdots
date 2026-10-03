use std::path::PathBuf;

use crate::entry::Entry;
use crate::fs;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortKey {
    Name,
    Size,
    Mtime,
    /// By file extension (case-insensitive), then name.
    Type,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewMode {
    List,
    #[allow(dead_code)]
    Grid,
    #[allow(dead_code)]
    Compact,
}

pub const HIST_MAX: usize = 32;

pub struct Tab {
    pub id: i32,
    pub cwd: PathBuf,
    /// Set for virtual listings (Recent/Trash); `cwd` keeps the previous real
    /// dir so history/back-navigation still behave.
    pub virt: Option<crate::fs::VirtualKind>,
    pub entries: Vec<Entry>,
    pub vis: Vec<usize>,
    pub sel: usize,
    /// True when no entry is active (fresh folders show no highlight).
    pub sel_none: bool,
    /// Name of the entry to highlight once the next listing lands (restores
    /// selection when navigating back to a parent folder).
    pub pending_sel: Option<String>,
    pub scroll: usize,
    pub filter: String,
    pub filter_regex: bool,
    pub filter_re: Option<regex::Regex>,
    pub view: ViewMode,
    pub sort_key: SortKey,
    pub sort_desc: bool,
    pub busy: bool,
    pub loading: bool,
    pub hist: Vec<PathBuf>,
    pub hist_pos: i64,
    /// Split view on for this tab: the right pane shows `pane` (an embedded
    /// tab that is *not* listed in the tab bar).
    pub split: bool,
    pub pane: Option<Box<Tab>>,
}

impl Tab {
    pub fn new(id: i32, cwd: PathBuf) -> Self {
        let mut t = Tab {
            id,
            cwd,
            virt: None,
            entries: Vec::new(),
            vis: Vec::new(),
            sel: 0,
            sel_none: true,
            pending_sel: None,
            scroll: 0,
            filter: String::new(),
            filter_regex: false,
            filter_re: None,
            view: ViewMode::List,
            sort_key: SortKey::Name,
            sort_desc: false,
            busy: false,
            loading: true,
            hist: Vec::new(),
            hist_pos: -1,
            split: false,
            pane: None,
        };
        t.push_history();
        t
    }

    pub fn nvis(&self) -> usize {
        self.vis.len()
    }

    pub fn is_virtual(&self) -> bool {
        self.virt.is_some()
    }

    /// Path for an entry: real_path for virtual listings, else cwd+name.
    pub fn entry_path(&self, e: &Entry) -> PathBuf {
        e.real_path
            .clone()
            .unwrap_or_else(|| crate::fs::full_path(&self.cwd, &e.name))
    }

    /// Cycle list → grid → compact → list. Port of C `toggle_view`.
    pub fn toggle_view(&mut self) {
        self.view = match self.view {
            ViewMode::List => ViewMode::Grid,
            ViewMode::Grid => ViewMode::Compact,
            ViewMode::Compact => ViewMode::List,
        };
        if self.sel >= self.vis.len() {
            self.sel = self.vis.len().saturating_sub(1);
        }
    }

    /// Cycle name → size → mtime → type → name. Port of C `cycle_sort`.
    pub fn cycle_sort(&mut self) {
        self.sort_key = match self.sort_key {
            SortKey::Name => SortKey::Size,
            SortKey::Size => SortKey::Mtime,
            SortKey::Mtime => SortKey::Type,
            SortKey::Type => SortKey::Name,
        };
    }

    pub fn selected(&self) -> Option<&Entry> {
        if self.sel_none || self.vis.is_empty() {
            return None;
        }
        let i = self.vis.get(self.sel.min(self.vis.len() - 1))?;
        self.entries.get(*i)
    }

    pub fn push_history(&mut self) {
        if self.hist.last() == Some(&self.cwd) {
            return;
        }
        // truncate forward history
        self.hist.truncate(self.hist_pos.saturating_add(1) as usize);
        self.hist.push(self.cwd.clone());
        self.hist_pos = self.hist.len() as i64 - 1;
        if self.hist.len() > HIST_MAX {
            self.hist.remove(0);
            self.hist_pos -= 1;
        }
    }

    /// Alt+Left/Alt+Right history nav. Port of C `hist_back`.
    pub fn nav_back(&mut self) -> bool {
        if self.hist_pos > 0 {
            self.hist_pos -= 1;
            self.cwd = self.hist[self.hist_pos as usize].clone();
            true
        } else {
            false
        }
    }

    /// Alt+Left/Alt+Right history nav. Port of C `hist_fwd`.
    pub fn nav_fwd(&mut self) -> bool {
        if (self.hist_pos as usize) + 1 < self.hist.len() {
            self.hist_pos += 1;
            self.cwd = self.hist[self.hist_pos as usize].clone();
            true
        } else {
            false
        }
    }

    pub fn rebuild_visible(&mut self) {
        self.vis.clear();
        for i in 0..self.entries.len() {
            if fs::match_filter(&self.filter, self.filter_regex, &self.filter_re,
                                &self.entries[i].name)
            {
                self.vis.push(i);
            }
        }
        if !self.sel_none && self.sel >= self.vis.len() {
            self.sel = if self.vis.is_empty() { 0 } else { self.vis.len() - 1 };
        }
        if !self.sel_none && self.scroll > self.sel {
            self.scroll = self.sel;
        }
    }

    /// Drop the active row and any multi-select flags.
    pub fn clear_selection(&mut self) {
        self.sel = 0;
        self.sel_none = true;
        for e in self.entries.iter_mut() {
            e.selected = false;
        }
    }

    /// Select the entry named by `pending_sel` once the listing is applied
    /// (set by back-navigation). Falls back to nothing when not found.
    pub fn resolve_pending_sel(&mut self) {
        let Some(name) = self.pending_sel.take() else { return };
        for (vi, ei) in self.vis.iter().enumerate() {
            if self.entries[*ei].name == name {
                self.sel = vi;
                self.sel_none = false;
                for e in self.entries.iter_mut() {
                    e.selected = false;
                }
                self.entries[*ei].selected = true;
                return;
            }
        }
        self.sel_none = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_overflow_regression() {
        // hist_pos starts at -1; the first push must not overflow.
        let mut t = Tab::new(0, PathBuf::from("/tmp"));
        assert_eq!(t.hist, vec![PathBuf::from("/tmp")]);
        assert_eq!(t.hist_pos, 0);
        t.cwd = PathBuf::from("/");
        t.push_history();
        assert_eq!(t.hist.len(), 2);
        assert_eq!(t.hist_pos, 1);
        assert!(t.nav_back());
        assert_eq!(t.cwd, PathBuf::from("/tmp"));
        assert!(t.nav_fwd());
        assert_eq!(t.cwd, PathBuf::from("/"));
        assert!(!t.nav_fwd());
    }

    #[test]
    fn view_cycle() {
        let mut t = Tab::new(0, PathBuf::from("/"));
        assert_eq!(t.view, ViewMode::List);
        t.toggle_view();
        assert_eq!(t.view, ViewMode::Grid);
        t.toggle_view();
        assert_eq!(t.view, ViewMode::Compact);
        t.toggle_view();
        assert_eq!(t.view, ViewMode::List);
    }

    #[test]
    fn sort_cycle() {
        let mut t = Tab::new(0, PathBuf::from("/"));
        assert_eq!(t.sort_key, SortKey::Name);
        t.cycle_sort();
        assert_eq!(t.sort_key, SortKey::Size);
        t.cycle_sort();
        assert_eq!(t.sort_key, SortKey::Mtime);
        t.cycle_sort();
        assert_eq!(t.sort_key, SortKey::Type);
        t.cycle_sort();
        assert_eq!(t.sort_key, SortKey::Name);
    }
}
