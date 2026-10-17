use std::path::PathBuf;

use crate::config::Config;
use crate::entry::{EntryType, WFM_MAX_INPUT};
use crate::menu::{Menu, MenuId, MenuItem};
use crate::tab::{Tab, ViewMode};
use crate::{fs, layout, wl};

/// Display label of a default place row id (0 home … 5 trash, 6 File System).
pub fn place_label(p: u8) -> String {
    match p {
        0 => "Home".to_string(),
        1 => "Documents".to_string(),
        2 => "Downloads".to_string(),
        3 => "Music".to_string(),
        4 => "Videos".to_string(),
        5 => "Trash".to_string(),
        _ => "File System".to_string(),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InputMode {
    None,
    Filter,
    Search,
    /// Inline editable location bar (breadcrumb bar becomes a text field).
    Url,
    /// Legacy modal "Go to Location" dialog (superseded by the inline URL
    /// bar, kept for compatibility).
    #[allow(dead_code)]
    Location,
    Rename,
    NewDir,
    NewFile,
    /// DnD "move/copy to new folder" — asks for a folder name first.
    DndNewDir,
    #[allow(dead_code)]
    NewWindow,
    /// "Open With…" custom command (typed by the user).
    OpenWith,
    /// "Set as Default for this type" — stores the command per mime type.
    SetDefault,
}

/// A reversible file operation, pushed on the undo stack so Ctrl+Z can undo
/// the last copy or move (Dolphin-style).
#[derive(Clone, Debug)]
pub enum UndoOp {
    Copy { src: PathBuf, dst: PathBuf },
    Move { src: PathBuf, dst: PathBuf },
}

/// Properties dialog contents (F9 / Ctrl+Enter).
#[derive(Clone)]
pub struct Props {
    pub path: String,
    pub kind: String,
    pub size: u64,
    pub mtime: String,
    pub is_dir: bool,
    pub perm: String,
    pub owner: String,
    pub group: String,
    pub inode: u64,
    pub nlink: u64,
}

/// Configure Toolbar dialog state (working copies applied on Close).
#[derive(Clone)]
pub struct ToolbarDlg {
    pub items: Vec<crate::toolbar::ToolbarItem>,
    /// Keys of items currently unchecked (hidden).
    pub hidden: Vec<String>,
    pub sel: usize,
    pub scroll: i32,
}

impl ToolbarDlg {
    pub(crate) fn is_hidden(&self, key: &str) -> bool {
        self.hidden.iter().any(|k| k == key)
    }

    fn toggle(&mut self, idx: usize) {
        let Some(it) = self.items.get(idx) else { return };
        if *it == crate::toolbar::ToolbarItem::Location {
            return;
        }
        let key = crate::toolbar::item_key(it);
        if let Some(pos) = self.hidden.iter().position(|k| *k == key) {
            self.hidden.remove(pos);
        } else {
            self.hidden.push(key);
        }
    }

    fn move_sel(&mut self, delta: i32) {
        if self.items.len() < 2 {
            return;
        }
        let cur = self.sel;
        let nxt = cur as i32 + delta;
        if nxt < 0 || nxt >= self.items.len() as i32 {
            return;
        }
        let nxt = nxt as usize;
        self.items.swap(cur, nxt);
        self.sel = nxt;
    }
}

pub struct App {
    pub cfg: Config,
    pub text: Option<crate::text::Text>,
    pub cfg_path: Option<PathBuf>,
    pub tabs: Vec<Tab>,
    pub cur_tab: usize,
    pub width: u32,
    pub height: u32,
    pub dirty: bool,
    pub running: bool,
    pub status: String,
    pub err: String,
    pub free_bytes: i64,
    pub hover_row: i32,
    pub n_sel: usize,

    /* scrollbar drag */
    pub scroll_drag: bool,
    pub scroll_drag_off: i32,
    /// Pane (0/1) the scrollbar drag belongs to (split-safe).
    pub scroll_drag_pane: i32,

    /* sidebar resize drag */
    pub side_resize: bool,
    pub side_resize_hover: bool,
    /* sidebar vertical scroll (offset in whole rows) */
    pub side_scroll: usize,
    pub side_sdrag: bool,
    pub side_sdrag_off: i32,

    /* preview resize drag */
    pub preview_resize: bool,
    pub preview_resize_hover: bool,

    /* split pane resize drag */
    pub split_resize: bool,
    pub split_resize_hover: bool,
    /// Width of the left split pane (0 = default half).
    pub split_width: i32,

    /* hidden default places (restored via View → Hidden Places) */
    pub hidden_places: Vec<String>,
    /// Default-place key the current context menu acts on (for HidePlace).
    pub ctx_place_key: Option<String>,

    /* inline input */
    pub input_mode: InputMode,
    pub input: String,
    pub input_cursor: usize,
    pub input_prompt: String,

    /* URL bar edit (InputMode::Url) */
    /// Byte range of the URL text selection: (anchor, caret).
    pub url_sel: Option<(usize, usize)>,
    /// A mouse-drag selection is in progress.
    pub url_drag_sel: bool,
    /// Previous URL texts, so Ctrl+Z / menu undo restores edits.
    pub url_undo: Vec<String>,
    /// Reversible file operations (Ctrl+Z undoes the last copy/move).
    pub undo_stack: Vec<UndoOp>,

    /* properties dialog */
    pub props: Option<Props>,
    /// Active properties section: 0 general, 1 permissions, 2 checksum, 3 details.
    pub props_tab: i32,
    pub props_checksum: Option<(String, String, String)>,
    pub props_checksum_pending: bool,
    pub checksum_jobs: std::sync::mpsc::Sender<std::path::PathBuf>,
    pub checksum_rx: Option<calloop::channel::Channel<crate::worker::ChecksumResult>>,

    /* close-with-open-tabs confirmation dialog */
    pub confirm_close: bool,
    pub confirm_dont_ask: bool,

    /* "import Thunar custom actions" confirmation dialog */
    pub confirm_import: bool,
    pub import_pending: Vec<crate::actions::CustomAction>,

    /* "create folder?" dialog from the URL bar (path that does not exist) */
    pub confirm_mkdir: Option<String>,

    /* Configure Toolbar dialog (View → Configure Toolbar…) */
    pub toolbar_dlg: Option<ToolbarDlg>,

    /* URL-bar dropdown: visited locations + filesystem completions */
    pub url_history: Vec<String>,
    pub url_sug_list: Vec<String>,

    /* breadcrumb hover (path segments) */
    pub breadcrumb_hover: i32,

    /* path-bar nav button hover */
    pub nav_hover: i32,

    /* sidebar */
    pub sidebar_visible: bool,
    pub side_hover: i32,
    pub side_n: usize,
    pub side_paths: Vec<PathBuf>,
    /// Source device (/dev/…) of each mounted volume, parallel to side_paths.
    pub side_devs: Vec<String>,
    /// Filesystem type of each mounted volume, parallel to side_paths.
    pub side_fstypes: Vec<String>,
    /// Filesystem label of each mounted volume ("" when none), parallel.
    pub side_labels: Vec<String>,
    pub side_free: Vec<i64>,
    /// Source device of the root ("/") mount (for its label + tooltip).
    pub root_dev: String,
    /// Label of the root partition ("" when none → show "/").
    pub root_label: String,
    pub net_n: usize,
    pub net_paths: Vec<PathBuf>,
    pub unmounted_n: usize,
    pub unmounted_paths: Vec<PathBuf>,
    /// Precomputed sidebar labels for unmounted volumes (size + label /
    /// fstype + " (Removable)"); built once per sidebar refresh.
    pub unmounted_labels: Vec<String>,
    /// Removable media (mounted first, then unmounted) — their own sidebar
    /// section below Volumes. `removable_paths[i]` is the mount point for a
    /// mounted device or the /dev/… path for an unmounted one.
    pub removable_n: usize,
    pub removable_paths: Vec<PathBuf>,
    /// Whether each removable entry is currently mounted (parallel to
    /// removable_paths).
    pub removable_mounted: Vec<bool>,
    /// Source device (/dev/…) of each removable entry, parallel.
    pub removable_devs: Vec<String>,
    /// Label (or "") of each removable entry, parallel.
    pub removable_labels: Vec<String>,
    /// Free bytes of mounted removable entries (parallel; -1 when unmounted).
    pub removable_free: Vec<i64>,
    pub bookmark_n: usize,
    pub bookmark_paths: Vec<PathBuf>,

    /* hover tooltip (lines, x, y) */
    pub tooltip: Option<(Vec<String>, i32, i32)>,
    /* shift-click range anchor (vis index of last plain click) */
    pub range_anchor: i32,

    /* which pane a rubberband / hover belongs to */
    /// Pane (0/1) the rubberband started in (split-safe).
    pub rubber_pane: i32,
    pub hover_pane: i32,

    /* path the sidebar context menu acts on (mount/unmount/open/props) */
    pub ctx_path: Option<PathBuf>,

    /* preview pane */
    pub preview_visible: bool,

    /* internal clipboard (cut/copy) */
    pub clip_paths: Vec<PathBuf>,
    pub clip_cut: bool,

    /* async worker (None in tests / fallback) */
    pub worker: Option<std::sync::mpsc::Sender<crate::worker::Job>>,

    /* rubberband selection */
    pub rubber: Option<(i32, i32, i32, i32)>,
    pub rubber_additive: bool,

    /* cross-window drag & drop */
    pub dnd_active: bool,
    pub dnd_hover: bool,
    pub dnd_hover_row: i32,
    pub dnd_drop_dir: Option<PathBuf>,
    pub dnd_uri_paths: Vec<PathBuf>,
    /// Last drag position from the DnD `motion` handler (the authoritative
    /// pointer location during a drag).
    pub dnd_x: i32,
    pub dnd_y: i32,
    /// DnD "to new folder": 0 = move, 1 = copy.
    pub dnd_new_mode: i32,

    /* context / ribbon menu */
    pub menu: Option<Menu>,
    pub menu_hover: i32,
    pub ribbon_which: i32,
    /// Thunar-style type-to-select: the chars typed while idle, shown in a
    /// small box at the bottom right; selects the first matching entry.
    pub typeahead: String,
    /// theme specs listed in the Theme ribbon (external theme files)
    pub theme_list: Vec<String>,
    /// Last menu origin — submenus (Open With…) reuse the same spot.
    pub menu_x: i32,
    pub menu_y: i32,
    /// "Open With" picker entries: (display name, exec, terminal).
    pub open_with_list: Vec<(String, String, bool)>,
    /// Thunar-style custom actions shown in the current context menu:
    /// (name, command).
    pub custom_actions: Vec<(String, String)>,
    /// DnD: drag offer currently hovering the sidebar (drop = add shortcut).
    pub dnd_side_hover: bool,

    /* last pointer for hover without full redraw storms */
    pub ptr_x: i32,
    pub ptr_y: i32,

    /* double click */
    pub last_click_time: u32,
    pub last_click_x: i32,
    pub last_click_y: i32,

    /* modifiers (tracked by wl.rs) */
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl App {
    pub fn new(cfg: Config) -> Self {
        let sidebar_visible = cfg.sidebar;
        let preview_visible = cfg.show_preview;
        let (chk_job_tx, chk_job_rx) = std::sync::mpsc::channel::<std::path::PathBuf>();
        let (chk_res_tx, chk_res_rx) =
            calloop::channel::channel::<crate::worker::ChecksumResult>();
        std::thread::Builder::new()
            .name("wfm-checksum".into())
            .spawn(move || {
                while let Ok(p) = chk_job_rx.recv() {
                    let (md5, sha1, sha256) = fs::checksums(&p);
                    let _ = chk_res_tx.send(crate::worker::ChecksumResult {
                        path: p.display().to_string(),
                        md5,
                        sha1,
                        sha256,
                    });
                }
            })
            .expect("failed to spawn checksum thread");
        App {
            cfg,
            text: None,
            cfg_path: None,
            tabs: Vec::new(),
            cur_tab: 0,
            width: 900,
            height: 600,
            dirty: true,
            running: true,
            status: String::new(),
            err: String::new(),
            free_bytes: -1,
            hover_row: -1,
            n_sel: 0,
            scroll_drag: false,
            scroll_drag_off: 0,
            scroll_drag_pane: 0,
            side_resize: false,
            side_resize_hover: false,
            side_scroll: 0,
            side_sdrag: false,
            side_sdrag_off: 0,
            preview_resize: false,
            preview_resize_hover: false,
            split_resize: false,
            split_resize_hover: false,
            split_width: 0,
            hidden_places: Vec::new(),
            ctx_place_key: None,
            input_mode: InputMode::None,
            input: String::new(),
            input_cursor: 0,
            input_prompt: String::new(),
            url_sel: None,
            url_drag_sel: false,
            url_undo: Vec::new(),
            undo_stack: Vec::new(),
            props: None,
            props_tab: 0,
            props_checksum: None,
            props_checksum_pending: false,
            checksum_jobs: chk_job_tx,
            checksum_rx: Some(chk_res_rx),
            confirm_close: false,
            confirm_dont_ask: false,
            confirm_import: false,
            import_pending: Vec::new(),
            confirm_mkdir: None,
            toolbar_dlg: None,
            url_history: Vec::new(),
            url_sug_list: Vec::new(),
            breadcrumb_hover: -1,
            nav_hover: -1,
            sidebar_visible,
            side_hover: -1,
            side_n: 0,
            side_paths: Vec::new(),
            side_devs: Vec::new(),
            side_fstypes: Vec::new(),
            side_labels: Vec::new(),
            side_free: vec![-1; 64],
            root_dev: String::new(),
            root_label: String::new(),
            net_n: 0,
            net_paths: Vec::new(),
            unmounted_n: 0,
            unmounted_paths: Vec::new(),
            unmounted_labels: Vec::new(),
            removable_n: 0,
            removable_paths: Vec::new(),
            removable_mounted: Vec::new(),
            removable_devs: Vec::new(),
            removable_labels: Vec::new(),
            removable_free: Vec::new(),
            bookmark_n: 0,
            bookmark_paths: Vec::new(),
            tooltip: None,
            range_anchor: -1,
            rubber_pane: 0,
            hover_pane: 0,
            ctx_path: None,
            preview_visible,
            clip_paths: Vec::new(),
            clip_cut: false,
            worker: None,
            rubber: None,
            rubber_additive: false,
            dnd_active: false,
            dnd_hover: false,
            dnd_hover_row: -1,
            dnd_drop_dir: None,
            dnd_uri_paths: Vec::new(),
            dnd_x: 0,
            dnd_y: 0,
            dnd_new_mode: 0,
            menu: None,
            menu_hover: -1,
            typeahead: String::new(),
            ribbon_which: -1,
            theme_list: Vec::new(),
            menu_x: 0,
            menu_y: 0,
            open_with_list: Vec::new(),
            custom_actions: Vec::new(),
            dnd_side_hover: false,
            ptr_x: 0,
            ptr_y: 0,
            last_click_time: 0,
            last_click_x: 0,
            last_click_y: 0,
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    pub fn tab(&self) -> &Tab {
        &self.tabs[self.cur_tab]
    }
    pub fn tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.cur_tab]
    }

    /// New tab (clone of the current cwd). Port of C `tab_add_clone`.
    pub fn new_tab(&mut self) {
        let cwd = self.tab().cwd.clone();
        self.new_tab_at(cwd);
    }

    /// New tab rooted at `dir` (used by "Open in New Tab" / split).
    pub fn new_tab_at(&mut self, dir: PathBuf) {
        if self.tabs.len() >= 8 {
            self.set_err("max tabs".into());
            return;
        }
        let id = self.tabs.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        let t = Tab::new(id, dir);
        self.tabs.push(t);
        self.cur_tab = self.tabs.len() - 1;
        self.reload();
        self.reload_status();
        self.dirty = true;
    }

    // ------------------------------------------------------------------
    // split pane (a per-tab right pane; the tab bar only shows real tabs)
    // ------------------------------------------------------------------

    /// Toggle split mode on the focused tab. Turning it on opens the current
    /// directory in an embedded right pane (not a separate tab).
    pub fn toggle_split(&mut self) {
        let on = self.tab().split;
        if on {
            self.tab_mut().split = false;
            self.save_state();
            self.dirty = true;
            return;
        }
        if self.tab().pane.is_none() {
            let id = self.next_tab_id();
            let t = self.tab_mut();
            t.split = true;
            t.pane = Some(Box::new(Tab::new(id, t.cwd.clone())));
        }
        let id = self.tab().pane.as_ref().unwrap().id;
        self.reload_pane_id(id);
        self.save_state();
        self.dirty = true;
    }

    /// Open `dir` in the focused tab's right split pane (replaces the pane's
    /// current folder). Pane is embedded in the tab — not a separate tab.
    pub fn open_split_at(&mut self, dir: PathBuf) {
        let need_new = self.tab().pane.is_none();
        let id = if need_new {
            let id = self.next_tab_id();
            let t = self.tab_mut();
            t.split = true;
            t.pane = Some(Box::new(Tab::new(id, dir)));
            id
        } else {
            let p = self.tab_mut().pane.as_mut().unwrap();
            p.cwd = dir;
            p.hist.clear();
            p.hist_pos = -1;
            p.push_history();
            p.entries.clear();
            p.sel = 0;
            p.sel_none = true;
            p.scroll = 0;
            p.id
        };
        self.reload_pane_id(id);
        self.save_state();
        self.dirty = true;
    }

    /// Make the right split pane the focused (left) pane. The panes swap, so
    /// the embedded right pane becomes the tab and the old tab moves right —
    /// the tab's identity/focus follows the panes, and keyboard input keeps
    /// operating on the focused pane.
    pub fn focus_right_pane(&mut self) {
        if !layout::split_active(self) {
            return;
        }
        let i = self.cur_tab;
        let Some(pane) = self.tabs[i].pane.take() else { return };
        let left = std::mem::replace(&mut self.tabs[i], *pane);
        let mut right = left;
        right.split = false;
        right.pane = None;
        self.tabs[i].split = true;
        self.tabs[i].pane = Some(Box::new(right));
        self.reload_status();
        self.dirty = true;
    }

    /// Tab of a split pane (0 = focused tab, 1 = its embedded right pane).
    pub fn pane_tab(&self, pane: i32) -> &Tab {
        if pane == 1 {
            if let Some(p) = &self.tabs[self.cur_tab].pane {
                return p;
            }
        }
        &self.tabs[self.cur_tab]
    }

    pub fn pane_tab_mut(&mut self, pane: i32) -> &mut Tab {
        let i = self.cur_tab;
        if pane == 1 && self.tabs[i].pane.is_some() {
            self.tabs[i].pane.as_mut().unwrap()
        } else {
            &mut self.tabs[i]
        }
    }

    /// Smallest free tab id (real tabs *and* embedded panes).
    fn next_tab_id(&self) -> i32 {
        let mut m = self.tabs.iter().map(|t| t.id).max().unwrap_or(0);
        for t in &self.tabs {
            if let Some(p) = &t.pane {
                m = m.max(p.id);
            }
        }
        m + 1
    }

    /// (Re)load the focused tab's embedded right pane's listing.
    pub fn reload_pane_id(&mut self, id: i32) {
        let (cwd, sort_key, sort_desc, virt) = {
            let t = self.tab();
            let Some(p) = t.pane.as_ref() else { return };
            (p.cwd.clone(), p.sort_key, p.sort_desc, p.virt)
        };
        if let Some(v) = virt {
            let p = self.tab_mut().pane.as_mut().unwrap();
            p.entries = crate::fs::virtual_entries(v);
            p.loading = false;
            p.rebuild_visible();
            self.dirty = true;
            return;
        }
        if let Some(tx) = self.worker.clone() {
            let _ = tx.send(crate::worker::Job::List {
                tab_id: id,
                cwd,
                dirs_first: self.cfg.dirs_first,
                show_hidden: self.cfg.show_hidden,
                sort_key,
                sort_desc,
            });
            self.tab_mut().pane.as_mut().unwrap().loading = true;
        } else {
            let dirs_first = self.cfg.dirs_first;
            let show_hidden = self.cfg.show_hidden;
            let p = self.tab_mut().pane.as_mut().unwrap();
            crate::fs::list_dir(p, dirs_first, show_hidden);
            p.loading = false;
            p.rebuild_visible();
            p.resolve_pending_sel();
        }
        self.dirty = true;
    }

    /// Close the focused tab (keeps at least one). Port of C `close_focused_tab`.
    pub fn close_tab(&mut self) {
        self.close_tab_at(self.cur_tab);
    }

    /// Confirm the pending Thunar import: overwrite wfm's own action file
    /// with the staged actions and report the result.
    pub fn do_import_actions(&mut self) {
        let actions = std::mem::take(&mut self.import_pending);
        self.confirm_import = false;
        match crate::actions::write_actions(&actions) {
            Ok(()) => self.set_status(format!(
                "imported {} custom action(s) from Thunar (replaced current)",
                actions.len()
            )),
            Err(e) => self.set_err(format!("import failed: {e}")),
        }
        self.dirty = true;
    }

    /// Window close request (Ctrl+Q / WM close / File→Quit). With multiple
    /// tabs open and `confirm_close` enabled, show a confirmation dialog.
    pub fn request_close(&mut self) {
        if self.tabs.len() > 1 && self.cfg.confirm_close {
            self.confirm_close = true;
            self.dirty = true;
        } else {
            self.running = false;
        }
    }

    /// Close the tab at `i` (keeps at least one).
    pub fn close_tab_at(&mut self, i: usize) {
        if self.tabs.len() <= 1 || i >= self.tabs.len() {
            return;
        }
        self.tabs.remove(i);
        if self.cur_tab > i {
            self.cur_tab -= 1;
        }
        self.cur_tab = self.cur_tab.min(self.tabs.len() - 1);
        self.reload_status();
        self.dirty = true;
    }

    pub fn select_tab(&mut self, i: usize) {
        if i >= self.tabs.len() {
            return;
        }
        self.cur_tab = i;
        self.reload_status();
        self.dirty = true;
    }

    pub fn next_tab(&mut self) {
        self.select_tab((self.cur_tab + 1) % self.tabs.len());
    }

    pub fn prev_tab(&mut self) {
        self.select_tab((self.cur_tab + self.tabs.len() - 1) % self.tabs.len());
    }

    /// Re-apply sort order after key changes. Port of C `cycle_sort`.
    pub fn apply_sort(&mut self) {
        let (key, desc) = {
            let t = self.tab();
            (t.sort_key, t.sort_desc)
        };
        fs::sort_entries(&mut self.tab_mut().entries, true, key, desc);
        let t = self.tab_mut();
        t.rebuild_visible();
        self.dirty = true;
    }

    /// Set the sort key (Dolphin "Sort By" menu); clicking the current key
    /// flips the direction.
    fn set_sort(&mut self, key: crate::tab::SortKey) {
        let t = self.tab_mut();
        if t.sort_key == key {
            t.sort_desc = !t.sort_desc;
        } else {
            t.sort_key = key;
            t.sort_desc = false;
        }
        self.apply_sort();
        self.save_state();
    }

    fn toggle_sort_desc(&mut self) {
        let t = self.tab_mut();
        t.sort_desc = !t.sort_desc;
        self.apply_sort();
        self.save_state();
    }

    /// Dolphin-style "Sort By" submenu: Name / Size / Type / Modified with the
    /// active key marked, plus a Descending toggle. Shared by the right-click
    /// menu and the View ribbon (opens to the side, not merged inline).
    fn sort_menu(&self) -> Menu {
        use crate::tab::SortKey as K;
        let t = self.tab();
        let mut v = Vec::new();
        let key = t.sort_key;
        let mark = |active: bool| if active { "● " } else { "  " };
        v.push(Menu::item(
            MenuId::SortName,
            format!("{}Name", mark(key == K::Name)),
            true,
        ));
        v.push(Menu::item(
            MenuId::SortSize,
            format!("{}Size", mark(key == K::Size)),
            true,
        ));
        v.push(Menu::item(
            MenuId::SortType,
            format!("{}Type", mark(key == K::Type)),
            true,
        ));
        v.push(Menu::item(
            MenuId::SortMtime,
            format!("{}Modified", mark(key == K::Mtime)),
            true,
        ));
        v.push(Menu::sep());
        v.push(Menu::item(
            MenuId::SortDesc,
            if t.sort_desc { "● Descending" } else { "Descending" },
            true,
        ));
        Menu { x: 0, y: 0, items: v, hover: -1, is_context: true, sub_open: None }
    }

    /// The "Sort By" opener item carrying the sort submenu.
    fn sort_submenu_item(&self) -> MenuItem {
        Menu::submenu(MenuId::SortBy, "Sort By", self.sort_menu())
    }

    pub fn toggle_hidden(&mut self) {
        self.cfg.show_hidden = !self.cfg.show_hidden;
        self.reload();
    }

    /// Navigate to a default place shortcut by key (home, documents,
    /// downloads, music, videos, trash). Trash is a virtual listing.
    pub fn place_cd(&mut self, key: &str) {
        match key {
            "home" => {
                let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"));
                self.cd(home);
            }
            "documents" => self.cd(self.place_dir("Documents")),
            "downloads" => self.cd(self.place_dir("Downloads")),
            "music" => self.cd(self.place_dir("Music")),
            "videos" => self.cd(self.place_dir("Videos")),
            "trash" => self.cd_virtual(fs::VirtualKind::Trash),
            _ => {}
        }
    }

    /// `$HOME/<sub>` (falls back to $HOME itself).
    fn place_dir(&self, sub: &str) -> PathBuf {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"));
        let d = home.join(sub);
        if d.is_dir() {
            d
        } else {
            home
        }
    }

    /// Path of a default place shortcut (home/documents/downloads/music/
    /// videos/trash), None for unknown keys. Trash is the virtual trash dir.
    pub fn place_path(&self, key: &str) -> Option<PathBuf> {
        match key {
            "home" => Some(
                std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/")),
            ),
            "documents" => Some(self.place_dir("Documents")),
            "downloads" => Some(self.place_dir("Downloads")),
            "music" => Some(self.place_dir("Music")),
            "videos" => Some(self.place_dir("Videos")),
            "trash" => Some(fs::trash_dir()),
            _ => None,
        }
    }

    /// Hide a default place shortcut from the sidebar (persisted).
    pub fn hide_place(&mut self, key: &str) {
        if layout::place_id(key) < 0 {
            return;
        }
        if !self.hidden_places.iter().any(|h| h == key) {
            self.hidden_places.push(key.to_string());
            self.set_status(format!("hidden {} from Places — restore via View → Hidden Places", key));
            self.save_state();
            self.dirty = true;
        }
    }

    /// Restore a hidden default place shortcut to the sidebar (persisted).
    pub fn show_place(&mut self, key: &str) {
        self.hidden_places.retain(|h| h != key);
        self.set_status(format!("restored {key} to Places"));
        self.save_state();
        self.dirty = true;
    }

    /// Update status (path, free space) for the current tab.
    pub fn reload_status(&mut self) {
        self.free_bytes = fs::free_bytes(&self.tab().cwd);
        let path = self.tab().cwd.display().to_string();
        self.set_status(path);
    }

    pub fn sidebar_refresh(&mut self) {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        self.side_free = vec![-1; 64];
        if let Some(ref h) = home {
            self.side_free[0] = fs::free_bytes(h);
        }
        self.side_free[1] = fs::free_bytes(std::path::Path::new("/"));
        self.side_paths.clear();
        self.side_devs.clear();
        self.side_fstypes.clear();
        self.side_labels.clear();
        self.side_n = 0;
        self.root_dev.clear();
        self.root_label.clear();
        // removable media (USB, card readers, …) go to their own section;
        // internal volumes stay under Volumes.
        self.removable_paths.clear();
        self.removable_mounted.clear();
        self.removable_devs.clear();
        self.removable_labels.clear();
        self.removable_free.clear();
        self.removable_n = 0;
        for m in fs::mounts(16) {
            if m.mp.as_os_str() == "/" {
                self.root_dev = m.dev.clone();
                self.root_label = fs::vol_label(&m.dev);
                continue;
            }
            if home.as_ref() == Some(&m.mp) {
                continue;
            }
            let free = fs::free_bytes(&m.mp);
            if fs::is_removable(&m.dev) {
                self.removable_paths.push(m.mp);
                self.removable_mounted.push(true);
                self.removable_devs.push(m.dev.clone());
                self.removable_labels.push(fs::vol_label(&m.dev));
                self.removable_free.push(free);
                self.removable_n += 1;
            } else {
                self.side_paths.push(m.mp);
                self.side_devs.push(m.dev.clone());
                self.side_fstypes.push(m.fstype);
                self.side_labels.push(fs::vol_label(&m.dev));
                self.side_free[2 + self.side_n] = free;
                self.side_n += 1;
            }
            if self.side_n + self.removable_n >= 16 {
                break;
            }
        }
        self.net_paths = fs::network_mounts(16);
        self.net_n = self.net_paths.len();
        let unmounted = fs::unmounted_volumes(16);
        self.unmounted_paths.clear();
        self.unmounted_n = 0;
        for dev in unmounted {
            if fs::is_removable(&dev.display().to_string()) {
                self.removable_paths.push(dev.clone());
                self.removable_mounted.push(false);
                self.removable_devs.push(dev.display().to_string());
                self.removable_labels
                    .push(fs::unmounted_volume_label(&dev.display().to_string()));
                self.removable_free.push(-1);
                self.removable_n += 1;
            } else {
                self.unmounted_paths.push(dev);
                self.unmounted_n += 1;
            }
            if self.unmounted_n + self.removable_n >= 16 {
                break;
            }
        }
        self.unmounted_labels = fs::unmounted_volume_labels(&self.unmounted_paths);
        // keep the sidebar scroll in range if the content shrank
        let max = layout::side_scroll_max(self);
        if self.side_scroll > max {
            self.side_scroll = max;
        }
        self.dirty = true;
    }

    /// Sidebar row ids: 0 Home, 1 Documents, 2 Downloads, 3 Music,
    /// 4 Videos, 5 Trash, 6 File System, 10+i volumes, 100+i network,
    /// 200+i unmounted, 300+i bookmarks, 400+i removable media (mounted or
    /// unmounted — the label row shows which).
    pub fn sidebar_nav(&mut self, row: i32) {
        match row {
            0 => self.place_cd("home"),
            1 => self.place_cd("documents"),
            2 => self.place_cd("downloads"),
            3 => self.place_cd("music"),
            4 => self.place_cd("videos"),
            5 => self.place_cd("trash"),
            6 => self.cd(PathBuf::from("/")),
            r if r >= 400 => {
                let u = (r - 400) as usize;
                if u < self.removable_n {
                    if self.removable_mounted[u] {
                        let p = self.removable_paths[u].clone();
                        self.cd(p);
                    } else {
                        let dev = self.removable_paths[u].clone();
                        let name = dev
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "vol".into());
                        let mp = PathBuf::from(format!("/media/{name}"));
                        self.set_status(format!("mounting {}…", dev.display()));
                        if fs::mount_with_polkit(&dev, &mp) {
                            self.sidebar_refresh();
                            self.cd(mp);
                            self.set_status(format!("mounted {}", dev.display()));
                        } else {
                            self.set_status(format!("failed to mount {} (polkit?)", dev.display()));
                        }
                    }
                }
            }
            r if r >= 300 => {
                let b = (r - 300) as usize;
                if b < self.bookmark_n {
                    let p = self.bookmark_paths[b].clone();
                    self.cd(p);
                }
            }
            r if r >= 200 => {
                let u = (r - 200) as usize;
                if u < self.unmounted_n {
                    let dev = self.unmounted_paths[u].clone();
                    let name = dev
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "vol".into());
                    let mp = PathBuf::from(format!("/media/{name}"));
                    self.set_status(format!("mounting {}…", dev.display()));
                    if fs::mount_with_polkit(&dev, &mp) {
                        self.sidebar_refresh();
                        self.cd(mp);
                        self.set_status(format!("mounted {}", dev.display()));
                    } else {
                        self.set_status(format!("failed to mount {} (polkit?)", dev.display()));
                    }
                }
            }
            r if r >= 100 => {
                let n = (r - 100) as usize;
                if n < self.net_n {
                    let p = self.net_paths[n].clone();
                    self.cd(p);
                }
            }
            r => {
                let m = (r - 10) as usize;
                if m < self.side_n {
                    let p = self.side_paths[m].clone();
                    self.cd(p);
                }
            }
        }
    }

    fn bookmarks_path() -> PathBuf {
        dirs::config_dir()
            .map(|p| p.join("wfm/bookmarks"))
            .unwrap_or_else(|| PathBuf::from("/tmp/wfm-bookmarks"))
    }

    pub fn load_bookmarks(&mut self) {
        self.bookmark_paths.clear();
        self.bookmark_n = 0;
        let Ok(s) = std::fs::read_to_string(Self::bookmarks_path()) else {
            return;
        };
        for line in s.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            self.bookmark_paths.push(PathBuf::from(line));
            self.bookmark_n += 1;
            if self.bookmark_n >= 16 {
                break;
            }
        }
    }

    pub fn save_bookmarks(&self) {
        let path = Self::bookmarks_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let body: String = self
            .bookmark_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("
");
        let _ = std::fs::write(path, body + "
");
    }

    pub fn toggle_shortcut_cwd(&mut self) {
        let cwd = self.tab().cwd.clone();
        self.toggle_shortcut_path(cwd);
    }

    pub fn toggle_shortcut_path(&mut self, path: PathBuf) {
        if let Some(i) = self.bookmark_paths.iter().position(|p| p == &path) {
            self.bookmark_paths.remove(i);
            self.bookmark_n = self.bookmark_paths.len();
            self.save_bookmarks();
            self.set_status(format!("removed shortcut: {}", path.display()));
            return;
        }
        if self.bookmark_n >= 16 {
            self.set_err("shortcut limit (16)".into());
            return;
        }
        // Only directories (or symlinks resolving to a directory) are valid
        // sidebar shortcuts — files, regular-file symlinks and broken
        // symlinks are rejected. `Path::is_dir` follows symlinks.
        if !path.is_dir() {
            self.set_err(format!(
                "not a folder — only directories can be added to Places: {}",
                path.display()
            ));
            return;
        }
        self.bookmark_paths.push(path.clone());
        self.bookmark_n = self.bookmark_paths.len();
        self.save_bookmarks();
        self.set_status(format!("added shortcut: {}", path.display()));
    }

    fn nav_action(&mut self, which: i32) {
        match which {
            0 => {
                if self.nav_back() {
                    self.reload();
                    self.reload_status();
                }
            }
            1 => {
                if self.nav_fwd() {
                    self.reload();
                    self.reload_status();
                }
            }
            2 => self.cd_parent(),
            3 => {
                let home = std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("/"));
                self.cd(home);
            }
            _ => {}
        }
    }

    /// Open the properties dialog for the selected entry. Port of C `open_props_selected`.
    pub fn open_props_selected(&mut self) {
        let (kind, size, mtime, is_dir) = {
            let t = self.tab();
            let Some(e) = t.selected() else { return };
            let kind = match e.kind {
                EntryType::Dir => "directory",
                EntryType::Link => "symlink",
                EntryType::Image => "image",
                EntryType::Archive => "archive",
                EntryType::File => {
                    if e.is_hardlink {
                        "hardlink"
                    } else {
                        "file"
                    }
                }
            };
            (kind.to_string(), e.size, e.mtime, e.is_dir)
        };
        let full = fs::full_path(&self.tab().cwd, &self.tab().selected().unwrap().name);
        self.open_props_path(&full, kind, size, mtime, is_dir);
    }

    /// Open the properties dialog for an arbitrary path (selection or a
    /// sidebar volume / bookmark).
    pub fn open_props_path(&mut self, full: &std::path::Path, kind: String, size: u64, mtime: i64, is_dir: bool) {
        use std::os::unix::fs::MetadataExt;
        let (perm, owner, group, inode, nlink) = match std::fs::symlink_metadata(full) {
            Ok(m) => (
                fs::fmt_mode(m.mode(), m.is_dir()),
                fs::user_name(m.uid()),
                fs::group_name(m.gid()),
                m.ino(),
                m.nlink(),
            ),
            Err(_) => ("?".into(), "?".into(), "?".into(), 0, 0),
        };
        self.props = Some(Props {
            path: full.display().to_string(),
            kind,
            size,
            mtime: crate::render::fmt_mtime(mtime),
            is_dir,
            perm,
            owner,
            group,
            inode,
            nlink,
        });
        self.props_tab = 0;
        self.props_checksum = None;
        if !is_dir && size < 512 * 1024 * 1024 {
            self.props_checksum_pending = true;
            let _ = self.checksum_jobs.send(full.to_path_buf());
        } else {
            self.props_checksum_pending = false;
        }
        self.dirty = true;
    }

    /// Properties for a path that is not in the current listing (sidebar
    /// volumes, bookmarks, …) — stats it on the spot.
    pub fn open_props_for(&mut self, path: &std::path::Path) {
        use std::os::unix::fs::MetadataExt;
        let (size, mtime, is_dir) = match std::fs::symlink_metadata(path) {
            Ok(m) => (m.len(), m.mtime(), m.is_dir()),
            Err(_) => (0, 0, true),
        };
        let kind = if is_dir { "directory" } else { "file" }.to_string();
        self.open_props_path(path, kind, size, mtime, is_dir);
    }

    /// Apply an async checksum result to the properties dialog.
    pub fn apply_checksum_result(&mut self, r: crate::worker::ChecksumResult) {
        if let Some(p) = &self.props {
            if p.path == r.path {
                self.props_checksum = Some((r.md5, r.sha1, r.sha256));
                self.props_checksum_pending = false;
                self.dirty = true;
            }
        }
    }

    pub fn cd(&mut self, dir: PathBuf) {
        self.cd_with_sel(dir, None);
    }

    /// Navigate and optionally highlight `name` once the listing lands (used
    /// by back-navigation so the folder you came from stays selected).
    pub fn cd_with_sel(&mut self, dir: PathBuf, name: Option<String>) {
        self.remember_url(&dir);
        {
            let t = self.tab_mut();
            t.cwd = dir;
            t.virt = None;
            t.loading = true;
            t.push_history();
            t.clear_selection();
            t.pending_sel = name;
            t.scroll = 0;
        }
        self.reload();
        self.free_bytes = fs::free_bytes(&self.tab().cwd);
        let path = self.tab().cwd.display().to_string();
        self.set_status(path);
        self.sidebar_refresh();
    }

    /// Enter a virtual listing (Recent / Trash) without touching real cwd or
    /// history. Port of the C sidebar Recent/Trash navigation.
    pub fn cd_virtual(&mut self, kind: fs::VirtualKind) {
        let entries = fs::virtual_entries(kind);
        let t = self.tab_mut();
        t.virt = Some(kind);
        t.entries = entries;
        t.loading = false;
        t.busy = false;
        t.clear_selection();
        t.pending_sel = None;
        t.scroll = 0;
        t.filter.clear();
        t.filter_regex = false;
        t.rebuild_visible();
        self.n_sel = 0;
        self.free_bytes = -1;
        self.set_status(match kind {
            fs::VirtualKind::Recent => "Recent Files".into(),
            fs::VirtualKind::Trash => "Trash".into(),
        });
    }

    /// App-level history back (clears virtual listing state).
    pub fn nav_back(&mut self) -> bool {
        let name = self
            .tab()
            .cwd
            .file_name()
            .map(|s| s.to_string_lossy().to_string());
        let ok = self.tab_mut().nav_back();
        if ok {
            let t = self.tab_mut();
            t.virt = None;
            t.pending_sel = name;
            t.sel_none = true;
            t.scroll = 0;
        }
        ok
    }

    /// App-level history forward (clears virtual listing state).
    pub fn nav_fwd(&mut self) -> bool {
        let name = self
            .tab()
            .cwd
            .file_name()
            .map(|s| s.to_string_lossy().to_string());
        let ok = self.tab_mut().nav_fwd();
        if ok {
            let t = self.tab_mut();
            t.virt = None;
            t.pending_sel = name;
            t.sel_none = true;
            t.scroll = 0;
        }
        ok
    }

    pub fn cd_parent(&mut self) {
        let name = self
            .tab()
            .cwd
            .file_name()
            .map(|s| s.to_string_lossy().to_string());
        let dir = fs::parent(&self.tab().cwd);
        self.cd_with_sel(dir, name);
    }

    pub fn open_selected(&mut self) {
        let (full, is_dir, is_link, virt) = {
            let t = self.tab();
            let Some(e) = t.selected() else { return };
            (t.entry_path(e), e.is_dir, e.is_link, t.is_virtual())
        };
        if is_link {
            // Symlinks follow their target inside wfm (never xdg-open), but
            // we navigate to the symlink's own path so that Back returns to
            // the directory that contained the link (Thunar-style), not the
            // target's parent.
            let target = fs::resolve_link(&full);
            match target {
                Some(t) if t.is_dir() => self.cd(full),
                Some(_) => {
                    if let Some(parent) = full.parent() {
                        self.cd(parent.to_path_buf());
                    }
                }
                None => self.set_err("broken symlink".into()),
            }
        } else if is_dir {
            self.cd(full);
        } else {
            // per-type default opener (set via Open With → Set as Default)
            // wins over the generic `opener`
            let mime = fs::mime_for_path(&full);
            let cmd = fs::default_app_for(&mime).unwrap_or_else(|| self.cfg.opener.clone());
            if fs::open_with_cmd(&full, &cmd).is_err() {
                self.set_err(format!("failed to launch {cmd}"));
            }
            if !virt {
                fs::recent_add(&full);
            }
        }
    }

    /// Persist session state (view/sort/panels/last dir + pane widths) so it
    /// survives restart.
    pub fn save_state(&self) {
        let s = crate::config::State {
            view: Some(self.tab().view),
            sort_key: Some(self.tab().sort_key),
            sort_desc: self.tab().sort_desc,
            sidebar: self.sidebar_visible,
            preview: self.preview_visible,
            split: self.tab().split,
            sidebar_width: Some(self.cfg.sidebar_width),
            preview_width: Some(self.cfg.preview_width),
            split_width: if self.split_width > 0 { Some(self.split_width) } else { None },
            hidden_places: self.hidden_places.clone(),
            last_dir: Some(self.tab().cwd.clone()),
        };
        let p = crate::config::state_path();
        s.save(&p);
    }

    /// Apply remembered session state to the initial tab.
    pub fn apply_state(&mut self, s: &crate::config::State) {
        if let Some(v) = s.view {
            self.tab_mut().view = v;
        }
        if let Some(k) = s.sort_key {
            self.tab_mut().sort_key = k;
        }
        self.tab_mut().sort_desc = s.sort_desc;
        if s.sidebar {
            self.sidebar_visible = true;
            self.cfg.sidebar = true;
        }
        if s.preview {
            self.preview_visible = true;
            self.cfg.show_preview = true;
        }
        if s.split {
            // split is per-tab: open an embedded right pane on the focused tab
            // (not a second tab). main() reloads its listing after app.cd().
            let needs_pane = self.tab().pane.is_none();
            let id = self.next_tab_id();
            let t = self.tab_mut();
            t.split = true;
            if needs_pane {
                let cwd = t.cwd.clone();
                t.pane = Some(Box::new(crate::tab::Tab::new(id, cwd)));
            }
        }
        if let Some(w) = s.sidebar_width {
            self.cfg.sidebar_width = w.clamp(80, 600);
        }
        if let Some(w) = s.preview_width {
            self.cfg.preview_width = w.clamp(100, 800);
        }
        if let Some(w) = s.split_width {
            self.split_width = w;
        }
        self.hidden_places = s.hidden_places.clone();
    }

    /// Re-read the config file + theme and apply them live (triggered only
    /// by `wfm --reload` → SIGUSR1, never by polling).
    pub fn reload_config(&mut self) {
        let Some(p) = self.cfg_path.clone() else {
            self.set_status("no config path".to_string());
            return;
        };
        let mut cfg = Config::default();
        crate::config::load(&mut cfg, &p);
        crate::theme::resolve(&cfg.theme).apply_to_cfg(&mut cfg);
        let font_changed =
            cfg.font_size != self.cfg.font_size || cfg.font_name != self.cfg.font_name;
        self.cfg = cfg;
        if font_changed {
            self.rebuild_text();
        }
        self.dirty = true;
        self.set_status(format!("config reloaded: {}", p.display()));
    }

    /// Enqueue an async listing for the current tab (sync fallback without
    /// a worker). Recent/Trash virtual views refresh synchronously.
    pub fn reload(&mut self) {
        // the listing changed, so a stale type-to-select match is invalid
        self.typeahead.clear();
        let (tab_id, cwd, dirs_first, show_hidden, sort_key, sort_desc) = {
            let t = self.tab();
            (
                t.id,
                t.cwd.clone(),
                self.cfg.dirs_first,
                self.cfg.show_hidden,
                t.sort_key,
                t.sort_desc,
            )
        };
        if let Some(kind) = self.tab().virt {
            let entries = fs::virtual_entries(kind);
            let t = self.tab_mut();
            t.entries = entries;
            t.loading = false;
            t.busy = false;
            t.rebuild_visible();
            self.count_selected();
            self.clamp_scroll();
            self.sidebar_refresh();
            self.dirty = true;
            return;
        }
        if let Some(tx) = &self.worker {
            let _ = tx.send(crate::worker::Job::List {
                tab_id,
                cwd,
                dirs_first,
                show_hidden,
                sort_key,
                sort_desc,
            });
            self.tab_mut().loading = true;
            self.dirty = true;
        } else {
            fs::list_dir(self.tab_mut(), dirs_first, show_hidden);
            let t = self.tab_mut();
            t.loading = false;
            t.rebuild_visible();
            t.resolve_pending_sel();
            self.count_selected();
            self.clamp_scroll();
            self.sidebar_refresh();
            self.dirty = true;
        }
    }

    /// Apply a worker result if it still matches the current tab state.
    /// Find a tab (real *or* embedded pane) by worker tab id.
    fn tab_mut_by_id(&mut self, id: i32) -> Option<&mut Tab> {
        let i = self.tabs.iter().position(|t| t.id == id);
        if let Some(i) = i {
            return Some(&mut self.tabs[i]);
        }
        let i = self
            .tabs
            .iter()
            .position(|t| t.pane.as_ref().is_some_and(|p| p.id == id))?;
        Some(self.tabs[i].pane.as_mut().unwrap())
    }

    pub fn apply_worker_result(&mut self, r: crate::worker::WResult) {
        match r {
            crate::worker::WResult::List { tab_id, cwd, entries } => {
                let is_pane = self
                    .tab()
                    .pane
                    .as_ref()
                    .map(|p| p.id == tab_id)
                    .unwrap_or(false);
                {
                    let Some(tab) = self.tab_mut_by_id(tab_id) else {
                        return;
                    };
                    if tab.is_virtual() || tab.cwd != cwd {
                        return; // stale result
                    }
                    tab.entries = entries;
                    tab.loading = false;
                    tab.rebuild_visible();
                    tab.resolve_pending_sel();
                }
                if is_pane {
                    let (x0, w) = layout::pane_geom(self, 1);
                    let max = layout::max_scroll_for(self, self.pane_tab(1), x0, w);
                    let p = self.pane_tab_mut(1);
                    if p.scroll > max {
                        p.scroll = max;
                    }
                    self.request_visible_thumbs_pane(1);
                } else {
                    self.n_sel = self.tab().entries.iter().filter(|e| e.selected).count();
                    self.clamp_scroll();
                    self.request_visible_thumbs();
                }
                self.dirty = true;
            }
            crate::worker::WResult::Thumb { tab_id, idx, name, rgba, w, h } => {
                let Some(tab) = self.tab_mut_by_id(tab_id) else {
                    return;
                };
                if idx >= tab.entries.len() {
                    return;
                }
                let e = &mut tab.entries[idx];
                if e.name != name {
                    return; // entry replaced
                }
                e.thumb = Some(rgba);
                e.thumb_w = w as i32;
                e.thumb_h = h as i32;
                e.thumb_pending = false;
                self.dirty = true;
            }
        }
    }

    /// Queue thumbnail decode jobs for visible image entries in grid view
    /// (focused/left pane).
    pub fn request_visible_thumbs(&mut self) {
        self.request_visible_thumbs_pane(0);
    }

    /// Queue thumbnail decode jobs for a split pane's grid view.
    pub fn request_visible_thumbs_pane(&mut self, pane: i32) {
        let Some(tx) = self.worker.clone() else { return };
        let t0 = self.pane_tab(pane);
        if t0.view != ViewMode::Grid || t0.is_virtual() {
            return;
        }
        let (_, w) = layout::pane_geom(self, pane);
        let cols = layout::grid_cols_in(self, w).max(1);
        let rows = layout::grid_rows_for(self, t0).max(1);
        let tab_id = t0.id;
        let cwd = t0.cwd.clone();
        let box_size = self.cfg.thumb_size as u32;
        let start = t0.scroll * cols;
        let end = (start + cols * rows).min(t0.nvis());
        let mut reqs = Vec::new();
        {
            let t = self.pane_tab(pane);
            for vi in start..end {
                if reqs.len() >= crate::entry::MAX_THUMB {
                    break;
                }
                let ei = t.vis[vi];
                let e = &t.entries[ei];
                if e.kind == EntryType::Image && e.thumb.is_none() && !e.thumb_pending {
                    reqs.push((ei, e.name.clone()));
                }
            }
        }
        for (ei, name) in reqs {
            let path = fs::full_path(&cwd, &name);
            if let Some(e) = self.pane_tab_mut(pane).entries.get_mut(ei) {
                e.thumb_pending = true;
            }
            let _ = tx.send(crate::worker::Job::Thumb {
                tab_id,
                idx: ei,
                path,
                name,
                box_size,
            });
        }
    }

    /// (Re)build the text engine from the current config (font name + size).
    pub fn rebuild_text(&mut self) {
        let family = if self.cfg.font_name.trim().is_empty() {
            None
        } else {
            Some(self.cfg.font_name.clone())
        };
        self.text = crate::text::Text::new_family(self.cfg.font_size as f32, family.as_deref());
    }

    pub fn set_status(&mut self, s: String) {
        self.status = s;
        self.err.clear();
        self.dirty = true;
    }

    pub fn set_err(&mut self, s: String) {
        self.err = s;
        self.status.clear();
        self.dirty = true;
    }

    pub fn count_selected(&mut self) {
        self.n_sel = self.tab().entries.iter().filter(|e| e.selected).count();
    }

    // ------------------------------------------------------------------
    // keyboard
    // ------------------------------------------------------------------

    pub fn on_key(&mut self, sym: wl::Keysym, utf8: Option<String>, ctrl: bool, shift: bool) {
        use wl::Keysym as K;

        if self.toolbar_dlg.is_some() {
            match sym {
                K::Escape => {
                    self.toolbar_dlg = None;
                    self.dirty = true;
                }
                K::Return | K::KP_Enter => self.apply_toolbar_dialog(),
                K::Up => {
                    if let Some(dlg) = self.toolbar_dlg.as_mut() {
                        if dlg.sel > 0 {
                            dlg.sel -= 1;
                        }
                    }
                    self.dirty = true;
                }
                K::Down => {
                    if let Some(dlg) = self.toolbar_dlg.as_mut() {
                        if dlg.sel + 1 < dlg.items.len() {
                            dlg.sel += 1;
                        }
                    }
                    self.dirty = true;
                }
                K::space => {
                    if let Some(dlg) = self.toolbar_dlg.as_mut() {
                        dlg.toggle(dlg.sel);
                    }
                    self.dirty = true;
                }
                _ => {}
            }
            return;
        }

        if self.confirm_close {
            match sym {
                K::Escape => {
                    self.confirm_close = false;
                    self.dirty = true;
                }
                K::Return | K::KP_Enter => self.running = false,
                _ => {}
            }
            return;
        }

        if self.confirm_import {
            match sym {
                K::Escape => {
                    self.confirm_import = false;
                    self.import_pending.clear();
                    self.dirty = true;
                }
                K::Return | K::KP_Enter => self.do_import_actions(),
                _ => {}
            }
            return;
        }

        if self.confirm_mkdir.is_some() {
            match sym {
                K::Escape => {
                    self.confirm_mkdir = None;
                    self.dirty = true;
                }
                K::Return | K::KP_Enter => self.do_create_url_folder(),
                _ => {}
            }
            return;
        }

        if self.menu.is_some() {
            if sym == K::Escape {
                self.menu = None;
                self.dirty = true;
            }
            return;
        }

        if self.props.is_some() {
            match sym {
                K::Escape => {
                    self.props = None;
                    self.dirty = true;
                }
                K::Left | K::KP_Left => {
                    self.props_tab = (self.props_tab + 3) % 4;
                    self.dirty = true;
                }
                K::Right | K::KP_Right => {
                    self.props_tab = (self.props_tab + 1) % 4;
                    self.dirty = true;
                }
                _ => {}
            }
            return;
        }

        if self.input_mode != InputMode::None {
            self.key_inline(sym, utf8, ctrl, shift);
            return;
        }

        if ctrl {
            self.key_ctrl(sym, shift);
            return;
        }

        if self.alt {
            match sym {
                K::Left | K::KP_Left => {
                    if self.nav_back() {
                        self.reload();
                    }
                    return;
                }
                K::Right | K::KP_Right => {
                    if self.nav_fwd() {
                        self.reload();
                    }
                    return;
                }
                _ => {}
            }
        }

        // Thunar-style type-to-select: while idle, typing selects the first
        // entry whose name starts with the typed text and shows it in a small
        // box at the bottom right (separate from the search bar). Enter opens
        // the match, Backspace shortens it, Esc dismisses it. While the box
        // is open every printable char appends; starting it only accepts
        // chars that aren't taken by the single-letter commands below.
        let starting = self.typeahead.is_empty();
        if let Some(s) = utf8 {
            let ch = s.chars().next();
            if let Some(ch) = ch {
                if ch.is_ascii_graphic() || ch == ' ' {
                    let allowed = if starting {
                        !self.reserved_typeahead_key(sym)
                    } else {
                        true
                    };
                    if allowed {
                        self.typeahead.push(ch);
                        self.typeahead_match();
                        return;
                    }
                }
            }
        }
        if !self.typeahead.is_empty() {
            match sym {
                K::Escape => {
                    self.typeahead.clear();
                    self.dirty = true;
                    return;
                }
                K::Return | K::KP_Enter => {
                    self.typeahead.clear();
                    self.dirty = true;
                    self.open_selected();
                    return;
                }
                K::BackSpace => {
                    self.typeahead.pop();
                    if self.typeahead.is_empty() {
                        self.dirty = true;
                    } else {
                        self.typeahead_match();
                    }
                    return;
                }
                _ => {}
            }
        }

        let rows = layout::visible_rows(self);
        match sym {
            // Esc always clears the selection in the idle state (menus /
            // dialogs / inline input close first in their own handlers).
            K::Escape => self.deselect_all(),
            K::q => self.request_close(),
            K::Up | K::KP_Up | K::k => self.move_sel_grid(0, -1),
            K::Down | K::KP_Down | K::j => self.move_sel_grid(0, 1),
            K::Left | K::KP_Left => {
                if self.tab().view == ViewMode::Grid {
                    self.move_sel_grid(-1, 0);
                } else {
                    self.cd_parent();
                }
            }
            K::Right | K::KP_Right => {
                if self.tab().view == ViewMode::Grid {
                    self.move_sel_grid(1, 0);
                } else {
                    self.open_selected();
                }
            }
            K::Page_Up | K::KP_Page_Up => self.sel_relative(-(rows as i64)),
            K::Page_Down | K::KP_Page_Down => self.sel_relative(rows as i64),
            K::Home => self.sel_to(0),
            K::End => {
                let t = self.tab_mut();
                t.sel = t.nvis().saturating_sub(1);
                t.sel_none = false;
                self.scroll_into_view();
            }
            K::Return | K::KP_Enter => self.open_selected(),
            K::BackSpace => self.cd_parent(),
            K::F3 | K::slash => self.begin_search(),
            K::n => self.begin_input(InputMode::NewDir, "", "New Folder"),
            K::r | K::F2 => {
                let name = self.tab().selected().map(|e| e.name.clone()).unwrap_or_default();
                if !name.is_empty() {
                    self.begin_input(InputMode::Rename, "", &name);
                }
            }
            K::b => {
                self.sidebar_visible = !self.sidebar_visible;
                self.dirty = true;
            }
            K::Tab => self.next_tab(),
            K::ISO_Left_Tab => self.prev_tab(),
            K::v => {
                let t = self.tab_mut();
                t.toggle_view();
                self.dirty = true;
                self.save_state();
                self.request_visible_thumbs();
            }
            K::s => {
                let t = self.tab_mut();
                t.cycle_sort();
                self.apply_sort();
                self.save_state();
            }
            K::S if shift => {
                let t = self.tab_mut();
                t.sort_desc = !t.sort_desc;
                self.apply_sort();
                self.save_state();
            }
            K::G if shift => self.cd(PathBuf::from("/")),
            K::h => self.toggle_hidden(),
            K::asciitilde => {
                let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"));
                self.cd(home);
            }
            K::F9 => self.open_props_selected(),
            K::F5 => self.toggle_split(),
            K::F4 => {
                self.sidebar_visible = !self.sidebar_visible;
                self.cfg.sidebar = self.sidebar_visible;
                self.dirty = true;
                self.save_state();
            }
            K::F8 => {
                self.preview_visible = !self.preview_visible;
                self.cfg.show_preview = self.preview_visible;
                self.dirty = true;
                self.save_state();
            }
            K::exclam if shift => {
                self.tab_mut().view = crate::tab::ViewMode::List;
                self.dirty = true;
                self.save_state();
            }
            K::at if shift => {
                self.tab_mut().view = crate::tab::ViewMode::Grid;
                self.dirty = true;
                self.save_state();
                self.request_visible_thumbs();
            }
            K::numbersign if shift => {
                self.tab_mut().view = crate::tab::ViewMode::Compact;
                self.dirty = true;
                self.save_state();
            }
            _ => {}
        }
    }

    fn key_ctrl(&mut self, sym: wl::Keysym, shift: bool) {
        use wl::Keysym as K;
        match sym {
            K::r => self.reload(),
            K::l => self.begin_url_edit(),
            K::a => self.select_all_visible(),
            K::h => self.toggle_hidden(),
            K::n if !shift => self.spawn_new_window(None),
            K::n if shift => self.begin_input(InputMode::NewDir, "", "New Folder"),
            K::t => self.new_tab(),
            K::w => self.close_tab(),
            K::q => self.running = false,
            K::x => self.clip_cut(),
            K::c => self.clip_copy(),
            K::v => self.clip_paste(),
            K::d => self.duplicate_selected(),
            K::z => self.undo_last(),
            K::plus | K::KP_Add | K::equal => self.zoom_icons(1),
            K::minus | K::KP_Subtract => self.zoom_icons(-1),
            K::Return | K::KP_Enter => self.open_props_selected(),
            _ => {}
        }
    }

    fn select_all_visible(&mut self) {
        let t = self.tab_mut();
        for i in t.vis.clone() {
            if let Some(e) = t.entries.get_mut(i) {
                e.selected = true;
            }
        }
        t.sel_none = false;
        self.count_selected();
        self.dirty = true;
    }

    pub fn spawn_new_window(&mut self, dir: Option<PathBuf>) {
        let dir = dir.unwrap_or_else(|| self.tab().cwd.clone());
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("wfm"));
        let _ = std::process::Command::new(exe)
            .arg("--new-window")
            .arg(&dir)
            .spawn();
        self.set_status(format!("new window: {}", dir.display()));
    }

    fn zoom_icons(&mut self, dir: i32) {
        let step = 8 * dir;
        self.cfg.grid_cell = (self.cfg.grid_cell + step).clamp(48, 256);
        self.cfg.thumb_size = (self.cfg.thumb_size + step).clamp(24, 128);
        // List/compact rows: grow the entry icon too. icon_size = 0 means
        // auto (font-derived); seed it on the first zoom so it reacts.
        let auto = self.text.as_ref().map(|t| t.line_h() * 3 / 4).unwrap_or(13);
        let cur = if self.cfg.icon_size > 0 {
            self.cfg.icon_size
        } else {
            auto
        };
        self.cfg.icon_size = (cur + step).clamp(8, 64);
        if let Some(p) = self.cfg_path.clone() {
            crate::config::set_keys(
                &p,
                &[
                    ("grid_cell", self.cfg.grid_cell.to_string()),
                    ("thumb_size", self.cfg.thumb_size.to_string()),
                    ("icon_size", self.cfg.icon_size.to_string()),
                ],
            );
        }
        self.dirty = true;
    }

    fn move_sel_grid(&mut self, dx: i32, dy: i32) {
        let cols = layout::grid_cols(self).max(1) as i32;
        let n = self.tab().nvis() as i32;
        if n == 0 {
            return;
        }
        // First item is a hard stop: Up / Left do nothing (no wrap-around,
        // no jump to the last cell of the row) so the selection stays put.
        if !self.tab().sel_none && self.tab().sel == 0 && (dx < 0 || dy < 0) {
            return;
        }
        let grid = self.tab().view == ViewMode::Grid;
        let mut sel = if self.tab().sel_none { -1 } else { self.tab().sel as i32 };
        if grid {
            let col = sel % cols;
            let row = sel / cols;
            let mut nc = col + dx;
            let mut nr = row + dy;
            // Horizontal moves wrap to the next/previous row (so holding the
            // right arrow keeps walking down the grid to the last item).
            if dx != 0 {
                if nc < 0 {
                    nr -= 1;
                    nc = cols - 1;
                } else if nc >= cols {
                    nr += 1;
                    nc = 0;
                }
            }
            if nr < 0 {
                nr = 0;
            }
            sel = nr * cols + nc;
        } else {
            sel += if dy != 0 { dy } else { dx };
        }
        if sel < 0 {
            sel = 0;
        }
        if sel >= n {
            sel = n - 1;
        }
        self.tab_mut().sel = sel as usize;
        self.tab_mut().sel_none = false;
        self.scroll_into_view();
    }

    fn key_inline(&mut self, sym: wl::Keysym, utf8: Option<String>, ctrl: bool, shift: bool) {
        if self.input_mode == InputMode::Url {
            return self.key_url_inline(sym, utf8, ctrl, shift);
        }
        use wl::Keysym as K;
        match sym {
            K::Escape => {
                if self.input_mode == InputMode::Search {
                    self.close_search();
                } else {
                    let was_dnd_new = self.input_mode == InputMode::DndNewDir;
                    self.input_mode = InputMode::None;
                    self.input.clear();
                    self.input_cursor = 0;
                    if was_dnd_new {
                        self.dnd_clear();
                    }
                    self.dirty = true;
                }
            }
            K::Return | K::KP_Enter => self.commit_input(),
            K::BackSpace => {
                if self.input_cursor > 0 {
                    self.input.remove(self.input_cursor - 1);
                    self.input_cursor -= 1;
                    self.input_changed();
                }
            }
            K::Left => {
                if self.input_cursor > 0 {
                    self.input_cursor -= 1;
                    self.dirty = true;
                }
            }
            K::Right => {
                if self.input_cursor < self.input.len() {
                    self.input_cursor += 1;
                    self.dirty = true;
                }
            }
            K::a if ctrl => {
                self.input_cursor = self.input.len();
                self.dirty = true;
            }
            _ => {
                if let Some(u) = utf8 {
                    if !u.is_empty() && self.input.len() < WFM_MAX_INPUT {
                        self.input.insert_str(self.input_cursor, &u);
                        self.input_cursor += u.len();
                        self.input_changed();
                    }
                }
            }
        }
    }

    fn input_changed(&mut self) {
        if self.input_mode == InputMode::Filter || self.input_mode == InputMode::Search {
            let f = self.input.clone();
            fs::filter_set(self.tab_mut(), &f, false);
            let t = self.tab_mut();
            t.rebuild_visible();
        }
        self.dirty = true;
    }

    /// Key handling for the inline editable location bar.
    fn key_url_inline(&mut self, sym: wl::Keysym, utf8: Option<String>, ctrl: bool, shift: bool) {
        use wl::Keysym as K;
        let (lo, hi) = self.url_range();
        match sym {
            K::Escape => self.cancel_url_edit(),
            K::Return | K::KP_Enter => self.commit_input(),
            K::BackSpace => {
                if hi > lo {
                    self.url_replace_selection("");
                } else if self.input_cursor > 0 {
                    self.url_snapshot();
                    self.input.remove(self.input_cursor - 1);
                    self.input_cursor -= 1;
                    self.dirty = true;
                }
            }
            K::Delete | K::KP_Delete => {
                if hi > lo {
                    self.url_replace_selection("");
                } else if self.input_cursor < self.input.len() {
                    self.url_snapshot();
                    self.input.remove(self.input_cursor);
                    self.dirty = true;
                }
            }
            K::Left | K::KP_Left => {
                if shift && self.input_cursor > 0 {
                    let (a, _) = self.url_sel.unwrap_or((self.input_cursor, self.input_cursor));
                    self.input_cursor -= 1;
                    self.url_sel = Some((a, self.input_cursor));
                    self.dirty = true;
                } else if self.input_cursor > 0 {
                    self.input_cursor -= 1;
                    self.url_sel = None;
                    self.dirty = true;
                }
            }
            K::Right | K::KP_Right => {
                if shift && self.input_cursor < self.input.len() {
                    let (a, _) = self.url_sel.unwrap_or((self.input_cursor, self.input_cursor));
                    self.input_cursor += 1;
                    self.url_sel = Some((a, self.input_cursor));
                    self.dirty = true;
                } else if self.input_cursor < self.input.len() {
                    self.input_cursor += 1;
                    self.url_sel = None;
                    self.dirty = true;
                }
            }
            K::Home => {
                self.input_cursor = 0;
                if !shift {
                    self.url_sel = None;
                }
                self.dirty = true;
            }
            K::End => {
                self.input_cursor = self.input.len();
                if !shift {
                    self.url_sel = None;
                }
                self.dirty = true;
            }
            _ => {
                if ctrl {
                    match sym {
                        K::a => self.url_select_all(),
                        K::c => self.url_copy(),
                        K::x => self.url_cut(),
                        K::v => self.url_paste(),
                        K::z => self.url_undo_pop(),
                        K::u => self.url_clear(),
                        K::w => self.url_undo_pop(),
                        _ => {}
                    }
                    return;
                }
                if let Some(u) = utf8 {
                    if !u.is_empty() {
                        if hi > lo {
                            self.url_replace_selection(&u);
                        } else if self.input.len() < WFM_MAX_INPUT {
                            self.url_snapshot();
                            self.input.insert_str(self.input_cursor, &u);
                            self.input_cursor += u.len();
                            self.dirty = true;
                        }
                    }
                }
            }
        }
    }

    fn commit_input(&mut self) {
        let mode = self.input_mode;
        let value = self.input.clone();
        self.input_mode = InputMode::None;
        self.input.clear();
        self.input_cursor = 0;
        self.url_sel = None;
        self.url_undo.clear();
        match mode {
            InputMode::Filter | InputMode::Search => {
                let t = self.tab_mut();
                t.rebuild_visible();
            }
            InputMode::Url | InputMode::Location => {
                self.url_go(value.trim());
            }
            InputMode::Rename => {
                let old = self.tab().selected().map(|e| e.name.clone()).unwrap_or_default();
                if !old.is_empty() && !value.is_empty() && old != value {
                    let full = fs::full_path(&self.tab().cwd, &old);
                    if fs::rename_path(&full, &value).is_err() {
                        self.set_err("rename failed".into());
                    }
                    self.reload();
                }
            }
            InputMode::NewDir => {
                let cwd = self.tab().cwd.clone();
                let p = fs::full_path(&cwd, value.trim());
                if fs::mkdir_path(&p).is_err() {
                    self.set_err("mkdir failed".into());
                }
                self.reload();
            }
            InputMode::NewFile => {
                let cwd = self.tab().cwd.clone();
                let p = fs::full_path(&cwd, value.trim());
                if fs::create_file(&p).is_err() {
                    self.set_err("create failed".into());
                }
                self.reload();
            }
            InputMode::DndNewDir => {
                let name = value.trim().to_string();
                let dir =
                    self.dnd_drop_dir.clone().unwrap_or_else(|| self.tab().cwd.clone());
                if name.is_empty() {
                    self.set_err("empty folder name".into());
                } else {
                    let mut target = dir.join(&name);
                    if target.exists() {
                        target = fs::unique_copy_name(&dir, &name);
                    }
                    if fs::mkdir_path(&target).is_ok() {
                        // reuse dnd_execute flow, dropping into the new folder
                        let mode = if self.dnd_new_mode == 1 { 0 } else { 1 };
                        self.dnd_execute_into(target, mode);
                    } else {
                        self.set_err("mkdir failed".into());
                        self.dnd_clear();
                    }
                }
            }
            InputMode::NewWindow => {
                let cwd = self.tab().cwd.clone();
                let dir = if value.trim().is_empty() { cwd } else { PathBuf::from(value.trim()) };
                let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("wfm"));
                let _ = std::process::Command::new(exe).arg("--new-window").arg(dir).spawn();
            }
            InputMode::OpenWith => {
                let cmd = value.trim().to_string();
                let Some(path) = self.selected_paths().first().cloned() else {
                    self.set_err("no selection".into());
                    return;
                };
                if cmd.is_empty() {
                    self.set_err("empty command".into());
                } else if fs::open_with_cmd(&path, &cmd).is_err() {
                    self.set_err(format!("failed to launch {cmd}"));
                } else {
                    self.set_status(format!("open with: {cmd}"));
                    if !self.tab().is_virtual() {
                        fs::recent_add(&path);
                    }
                }
            }
            InputMode::SetDefault => {
                let cmd = value.trim().to_string();
                let mime = self
                    .tab()
                    .selected()
                    .map(|e| fs::mime_for_path(&fs::full_path(&self.tab().cwd, &e.name)))
                    .unwrap_or_default();
                if cmd.is_empty() || mime.is_empty() {
                    self.set_err("cannot set default".into());
                } else {
                    fs::set_default_app(&mime, &cmd);
                    self.set_status(format!("default opener for {mime}: {cmd}"));
                }
            }
            InputMode::None => {}
        }
        self.dirty = true;
    }

    /// Cancel the active input dialog without committing (Esc, Cancel button).
    fn cancel_input(&mut self) {
        let was_dnd_new = self.input_mode == InputMode::DndNewDir;
        self.input_mode = InputMode::None;
        self.input.clear();
        self.input_cursor = 0;
        self.url_sel = None;
        self.url_undo.clear();
        if was_dnd_new {
            self.dnd_clear();
        }
        self.dirty = true;
    }

    fn begin_input(&mut self, mode: InputMode, prompt: &str, init: &str) {
        self.typeahead.clear();
        self.input_mode = mode;
        self.input_prompt = prompt.to_string();
        self.input = init.chars().take(WFM_MAX_INPUT).collect();
        self.input_cursor = self.input.len();
        self.dirty = true;
    }

    /// Start inline editing of the location bar (no popup dialog).
    fn begin_url_edit(&mut self) {
        if self.input_mode == InputMode::Url {
            return;
        }
        if self.input_mode != InputMode::None {
            return;
        }
        let cwd = self.tab().cwd.display().to_string();
        self.begin_input(InputMode::Url, "", &cwd);
        self.input_cursor = self.input.len();
        self.url_sel = None;
        self.url_undo.clear();
    }

    /// Leave inline URL editing without navigating (Esc / click elsewhere).
    fn cancel_url_edit(&mut self) {
        self.cancel_input();
    }

    /// Byte range (lo, hi) of the URL selection (lo == hi for a caret).
    pub fn url_range(&self) -> (usize, usize) {
        let (a, b) = self.url_sel.unwrap_or((self.input_cursor, self.input_cursor));
        if a <= b {
            (a, b)
        } else {
            (b, a)
        }
    }

    /// Selection text in the URL field.
    pub fn url_selected_text(&self) -> String {
        let (lo, hi) = self.url_range();
        self.input[lo..hi.min(self.input.len())].to_string()
    }

    fn url_snapshot(&mut self) {
        self.url_undo.push(self.input.clone());
        if self.url_undo.len() > 64 {
            self.url_undo.remove(0);
        }
    }

    fn url_undo_pop(&mut self) {
        if let Some(prev) = self.url_undo.pop() {
            self.input = prev;
            self.input_cursor = self.input.len();
            self.url_sel = None;
            self.dirty = true;
        }
    }

    fn url_replace_selection(&mut self, repl: &str) {
        let (lo, hi) = self.url_range();
        self.url_snapshot();
        let mut s = self.input.clone();
        s.replace_range(lo..hi, repl);
        self.input = s.chars().take(WFM_MAX_INPUT).collect();
        self.input_cursor = lo + repl.len();
        self.url_sel = None;
        self.dirty = true;
    }

    fn url_copy(&mut self) {
        let text = self.url_selected_text();
        if text.is_empty() {
            return;
        }
        fs::clipboard_set_text(&text);
        self.set_status(format!("copied {} char(s)", text.chars().count()));
    }

    fn url_cut(&mut self) {
        let text = self.url_selected_text();
        if text.is_empty() {
            return;
        }
        fs::clipboard_set_text(&text);
        self.url_replace_selection("");
        self.set_status(format!("cut {} char(s)", text.chars().count()));
    }

    fn url_paste(&mut self) {
        let clip = fs::clipboard_get_text();
        if clip.is_empty() {
            return;
        }
        self.url_replace_selection(&clip);
    }

    fn url_clear(&mut self) {
        if self.input.is_empty() {
            return;
        }
        self.url_snapshot();
        self.input.clear();
        self.input_cursor = 0;
        self.url_sel = None;
        self.dirty = true;
    }

    fn url_select_all(&mut self) {
        self.url_sel = Some((0, self.input.len()));
        self.dirty = true;
    }

    /// Navigate to the typed location (tick button / Enter). A directory goes
    /// directly; a file opens its parent with it selected; a path that does
    /// not exist offers to create the folder (Dolphin-style).
    fn url_go(&mut self, value: &str) {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            self.set_status("enter a path".into());
            self.dirty = true;
            return;
        }
        let p = PathBuf::from(&trimmed);
        if p.is_dir() {
            self.cd(p);
            return;
        }
        if p.is_file() {
            if let Some(parent) = p.parent() {
                let name = p.file_name().map(|s| s.to_string_lossy().to_string());
                self.cd_with_sel(parent.to_path_buf(), name);
            }
            return;
        }
        self.confirm_mkdir = Some(trimmed);
        self.set_status("folder not found — create it?".into());
        self.dirty = true;
    }

    /// Record a visited location for the URL dropdown (most-recent first,
    /// deduped, capped).
    fn remember_url(&mut self, p: &std::path::Path) {
        let s = p.display().to_string();
        if let Some(pos) = self.url_history.iter().position(|h| h == &s) {
            self.url_history.remove(pos);
        }
        self.url_history.insert(0, s);
        self.url_history.truncate(30);
    }

    /// Filesystem completion for the URL dropdown: entries under the parent
    /// of the typed path whose name starts with the typed stem (dirs and
    /// files), empty input starts from the cwd.
    fn url_suggestions(&self) -> Vec<String> {
        let t = self.input.trim();
        let mut out = Vec::new();
        let (parent, prefix) = if t.is_empty() {
            (self.tab().cwd.clone(), String::new())
        } else {
            let p = PathBuf::from(t);
            let parent =
                p.parent().map(|q| q.to_path_buf()).unwrap_or_else(|| PathBuf::from("/"));
            let prefix =
                p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            (parent, prefix)
        };
        if let Ok(rd) = std::fs::read_dir(&parent) {
            for ent in rd.flatten() {
                let name = ent.file_name().to_string_lossy().to_string();
                if name.starts_with(&prefix) {
                    out.push(parent.join(&name).display().to_string());
                    if out.len() >= 12 {
                        break;
                    }
                }
            }
        }
        out.sort_by_key(|a| a.to_lowercase());
        out
    }

    /// Drop-down button under the URL field: history of visited locations
    /// plus completions for the typed text.
    pub fn open_url_dropdown(&mut self) {
        if self.input_mode != InputMode::Url {
            return;
        }
        let sug = self.url_suggestions();
        self.url_sug_list = sug;
        let (bx, by, _bw, bh) = {
            let r = layout::url_drop_rect(self);
            (r.x, r.y, r.w, r.h)
        };
        let mut items = Vec::new();
        if self.url_history.is_empty() && self.url_sug_list.is_empty() {
            items.push(Menu::item(MenuId::MenuHeader, "no matches", false));
        }
        if !self.url_history.is_empty() {
            items.push(Menu::item(MenuId::MenuHeader, "History", false));
            for (i, h) in self.url_history.iter().take(10).enumerate() {
                items.push(Menu::item(MenuId::UrlHistory(i as i32), h.clone(), true));
            }
        }
        if !self.url_sug_list.is_empty() {
            items.push(Menu::item(MenuId::MenuHeader, "Suggestions", false));
            for (i, s) in self.url_sug_list.iter().enumerate() {
                items.push(Menu::item(MenuId::UrlSuggest(i as i32), s.clone(), true));
            }
        }
        let mut m = Menu { x: bx, y: by + bh, items, hover: -1, is_context: false, sub_open: None };
        self.clamp_menu(&mut m);
        self.menu = Some(m);
        self.dirty = true;
    }

    /// Activate a dropdown pick: leave edit mode and navigate to the chosen
    /// path (same flow as Enter).
    fn url_go_from(&mut self, pick: Option<&String>) {
        self.url_sug_list.clear();
        let Some(p) = pick.cloned() else { return };
        self.input_mode = InputMode::None;
        self.input.clear();
        self.input_cursor = 0;
        self.url_sel = None;
        self.url_undo.clear();
        self.url_go(&p);
    }

    /// "Create" button on the folder-not-found dialog: make the folder and
    /// open it.
    fn do_create_url_folder(&mut self) {
        let Some(path) = self.confirm_mkdir.take() else { return };
        let p = PathBuf::from(&path);
        if std::fs::create_dir_all(&p).is_ok() {
            self.cd(p);
            self.set_status(format!("created folder {path}"));
        } else {
            self.set_err(format!("could not create {path}"));
        }
        self.dirty = true;
    }

    /// Thunar-style search: start with a fresh filter typed inline in the
    /// location bar, not a dialog popup.
    fn begin_search(&mut self) {
        if self.input_mode == InputMode::Search {
            return;
        }
        let t = self.tab_mut();
        fs::filter_set(t, "", false);
        t.rebuild_visible();
        self.begin_input(InputMode::Search, "search:", "");
    }

    /// Close the inline search (Esc / red X): clear the filter and exit.
    fn close_search(&mut self) {
        if self.input_mode != InputMode::Search {
            return;
        }
        self.input_mode = InputMode::None;
        self.input.clear();
        self.input_cursor = 0;
        let t = self.tab_mut();
        fs::filter_set(t, "", false);
        t.rebuild_visible();
        self.dirty = true;
    }

    /// Run a toolbar custom-action button (cwd as working dir, no selection).
    fn run_toolbar_action(&mut self, name: String) {
        let Some((_, cmd)) = self.custom_actions.iter().find(|(n, _)| *n == name).cloned() else {
            self.set_err(format!("custom action not found: {name}"));
            return;
        };
        let cwd = self.tab().cwd.clone();
        let expanded = crate::actions::expand_command(&cmd, &[]);
        let r = std::process::Command::new("sh")
            .arg("-c")
            .arg(&expanded)
            .current_dir(&cwd)
            .spawn();
        if r.is_err() {
            self.set_err(format!("failed to run custom action: {name}"));
        } else {
            self.set_status(format!("custom action: {name}"));
        }
    }

    pub fn scroll_into_view(&mut self) {
        let grid = self.tab().view == ViewMode::Grid;
        if grid {
            let cols = layout::grid_cols(self).max(1);
            let rows = layout::grid_rows(self).max(1);
            let t = self.tab_mut();
            let row = t.sel / cols;
            if row < t.scroll {
                t.scroll = row;
            } else if row >= t.scroll + rows {
                t.scroll = row + 1 - rows;
            }
        } else {
            let rows = layout::visible_rows(self);
            let t = self.tab_mut();
            if t.sel < t.scroll {
                t.scroll = t.sel;
            } else if rows > 0 && t.sel >= t.scroll + rows {
                t.scroll = t.sel + 1 - rows;
            }
        }
        self.dirty = true;
    }

    fn sel_relative(&mut self, d: i64) {
        let t = self.tab_mut();
        let n = t.nvis() as i64;
        if n == 0 {
            return;
        }
        let mut s = if t.sel_none { -1 } else { t.sel as i64 } + d;
        if s < 0 {
            s = 0;
        }
        if s >= n {
            s = n - 1;
        }
        t.sel = s as usize;
        t.sel_none = false;
        self.scroll_into_view();
    }

    fn sel_to(&mut self, idx: usize) {
        let t = self.tab_mut();
        t.sel = idx.min(t.nvis().saturating_sub(1));
        t.sel_none = false;
        t.scroll = t.sel;
        self.dirty = true;
    }

    /// Type-to-select: jump to the first visible entry whose name starts
    /// with the current `typeahead` text (case-insensitive) and select it.
    fn typeahead_match(&mut self) {
        let q = self.typeahead.to_lowercase();
        let t = self.tab_mut();
        let mut hit: Option<usize> = None;
        for (vi, &ei) in t.vis.iter().enumerate() {
            if t.entries[ei].name.to_lowercase().starts_with(&q) {
                hit = Some(vi);
                break;
            }
        }
        match hit {
            Some(vi) => {
                for e in t.entries.iter_mut() {
                    e.selected = false;
                }
                t.sel = vi;
                t.sel_none = false;
                if let Some(e) = t.entries.get_mut(t.vis[vi]) {
                    e.selected = true;
                }
            }
            None => {
                for e in t.entries.iter_mut() {
                    e.selected = false;
                }
                t.sel_none = true;
            }
        }
        self.count_selected();
        self.scroll_into_view();
    }

    /// Keys that must not start type-to-select while idle — they are taken
    /// by the single-letter / search commands in `on_key`.
    fn reserved_typeahead_key(&self, sym: wl::Keysym) -> bool {
        use wl::Keysym as K;
        matches!(
            sym,
            K::q | K::k | K::j | K::n | K::r | K::b | K::v | K::s | K::h
            | K::S | K::G
            | K::asciitilde | K::slash | K::F3
            | K::exclam | K::at | K::numbersign
        )
    }

    // ------------------------------------------------------------------
    // pointer
    // ------------------------------------------------------------------

    pub fn on_pointer_motion(&mut self, x: i32, y: i32) {
        if self.url_drag_sel {
            self.input_cursor = layout::url_caret_at(self, x);
            let anchor = self.url_sel.map(|(a, _)| a).unwrap_or(self.input_cursor);
            self.url_sel = Some((anchor, self.input_cursor));
            self.dirty = true;
            return;
        }
        if self.side_sdrag {
            layout::side_scroll_drag_to(self, y, self.side_sdrag_off);
            return;
        }
        if self.scroll_drag {
            self.scroll_drag_to(y);
            return;
        }
        if self.rubber.is_some() {
            self.ptr_x = x;
            self.ptr_y = y;
            self.update_rubber(x, y);
            return;
        }
        if self.side_resize {
            let w = layout::clamp_sidebar_width(self, x);
            if w != self.cfg.sidebar_width {
                self.cfg.sidebar_width = w;
                self.dirty = true;
            }
            return;
        }
        if self.preview_resize {
            // drag left edge of preview: width = window - pointer x
            let w = layout::clamp_preview_width(self, self.width as i32 - x);
            if w != self.cfg.preview_width {
                self.cfg.preview_width = w;
                self.dirty = true;
            }
            return;
        }
        if self.split_resize {
            let w = layout::clamp_split_width(self, x - layout::content_x(self));
            if w != self.split_width {
                self.split_width = w;
                self.dirty = true;
            }
            return;
        }
        self.ptr_x = x;
        self.ptr_y = y;
        // Ribbon sticky: once a top menu is open, hover switches File/View/Help like desktop UIs.
        if let Some(ref menu) = self.menu {
            if !menu.is_context {
                if let Some(ri) = layout::menu_bar_at(self, x, y) {
                    if self.ribbon_which != ri as i32 {
                        self.open_ribbon(ri);
                        return;
                    }
                }
            }
            self.menu_hover_at(x, y);
            // Keep tracking bar hover while dropdown is open (still return — list under menu inactive)
            return;
        }
        let side_resize_hover = layout::sidebar_splitter_at(self, x, y);
        let preview_resize_hover = !side_resize_hover && layout::preview_splitter_at(self, x, y);
        let split_resize_hover = !side_resize_hover && !preview_resize_hover && layout::split_splitter_at(self, x, y);
        let splitter = side_resize_hover || preview_resize_hover || split_resize_hover;
        let (hover_row, hover_pane) = if splitter {
            (-1, 0)
        } else if layout::split_active(self) && layout::pane_of(self, x, y) == 1 {
            let (px0, pw) = layout::pane_geom(self, 1);
            let t = self.pane_tab(1);
            (layout::vis_index_at_for(self, t, px0, pw, x, y), 1)
        } else {
            (layout::vis_index_at(self, x, y), 0)
        };
        let nav_hover = layout::nav_at(self, x, y);
        let side_hover = if splitter {
            -1
        } else {
            layout::sidebar_at(self, x, y)
        };
        if side_resize_hover != self.side_resize_hover
            || preview_resize_hover != self.preview_resize_hover
            || split_resize_hover != self.split_resize_hover
            || hover_row != self.hover_row
            || hover_pane != self.hover_pane
            || nav_hover != self.nav_hover
            || side_hover != self.side_hover
        {
            self.side_resize_hover = side_resize_hover;
            self.preview_resize_hover = preview_resize_hover;
            self.split_resize_hover = split_resize_hover;
            self.hover_row = hover_row;
            self.hover_pane = hover_pane;
            self.nav_hover = nav_hover;
            self.side_hover = side_hover;
            self.dirty = true;
        }
        // hover tooltip (sidebar places / volumes / bookmarks)
        let tt = self.sidebar_tooltip(x, y);
        let changed = match (&self.tooltip, &tt) {
            (Some(a), Some(b)) => a.0 != b.0,
            (None, None) => false,
            _ => true,
        };
        if changed {
            self.tooltip = tt;
            self.dirty = true;
        }
    }

    pub fn on_pointer_leave(&mut self) {
        self.url_drag_sel = false;
        self.hover_row = -1;
        self.hover_pane = 0;
        self.nav_hover = -1;
        self.side_hover = -1;
        self.side_resize_hover = false;
        self.preview_resize_hover = false;
        self.split_resize_hover = false;
        self.tooltip = None;
        // keep side_resize / preview_resize / split_resize / scroll_drag until button release if still held
        self.dirty = true;
    }

    /// Tooltip lines + position for the sidebar row under the pointer, or
    /// None when not hovering a row (or while a menu / dialog is open).
    fn sidebar_tooltip(&self, x: i32, y: i32) -> Option<(Vec<String>, i32, i32)> {
        if self.menu.is_some() || self.props.is_some() || self.dnd_active {
            return None;
        }
        let row = layout::sidebar_at(self, x, y);
        if row < 0 {
            return None;
        }
        let lines: Vec<String> = if row < 7 {
            match row {
                0 => {
                    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"));
                    vec![home.display().to_string()]
                }
                1 => vec![self.place_dir("Documents").display().to_string()],
                2 => vec![self.place_dir("Downloads").display().to_string()],
                3 => vec![self.place_dir("Music").display().to_string()],
                4 => vec![self.place_dir("Videos").display().to_string()],
                5 => vec!["Trash".into()],
                _ => self.volume_tooltip_lines("/", &self.root_dev),
            }
        } else if row >= 400 {
            let i = (row - 400) as usize;
            if i >= self.removable_n {
                return None;
            }
            if self.removable_mounted[i] {
                self.volume_tooltip_lines(
                    &self.removable_paths[i].display().to_string(),
                    &self.removable_devs[i],
                )
            } else {
                vec![self.removable_paths[i].display().to_string()]
            }
        } else if row >= 300 {
            let i = (row - 300) as usize;
            if i >= self.bookmark_n {
                return None;
            }
            vec![self.bookmark_paths[i].display().to_string()]
        } else if row >= 200 {
            let i = (row - 200) as usize;
            if i >= self.unmounted_n {
                return None;
            }
            vec![self.unmounted_paths[i].display().to_string()]
        } else if row >= 100 {
            let i = (row - 100) as usize;
            if i >= self.net_n {
                return None;
            }
            vec![self.net_paths[i].display().to_string()]
        } else {
            let i = (row - 10) as usize;
            if i >= self.side_n {
                return None;
            }
            let dev = self.side_devs.get(i).cloned().unwrap_or_default();
            self.volume_tooltip_lines(
                &self.side_paths[i].display().to_string(),
                &dev,
            )
        };
        // place near the pointer, clamped to the window
        let lh = layout::line_h(self);
        let max_w = lines.iter().map(|l| {
            self.text.as_ref().map(|t| t.width(l) as i32).unwrap_or(l.len() as i32 * 8)
        }).max().unwrap_or(100);
        let tw = max_w + self.cfg.padding * 2;
        let th = lines.len() as i32 * lh + self.cfg.padding;
        let tx = (x + 14).min((self.width as i32 - tw - 4).max(4));
        let ty = (y + 16).min((self.height as i32 - th - 4).max(4));
        Some((lines, tx, ty))
    }

    /// Tooltip for a mounted volume: device, filesystem, free/total and
    /// percent used. Falls back to the mount point when usage is unknown.
    fn volume_tooltip_lines(&self, mp: &str, dev: &str) -> Vec<String> {
        let (free, total) = fs::fs_usage(std::path::Path::new(mp));
        if free < 0 || total <= 0 {
            return vec![mp.to_string()];
        }
        let pct = ((total - free) * 100 / total).max(0);
        let f = crate::config::fmt_size(free);
        let t = crate::config::fmt_size(total);
        if dev.is_empty() {
            vec![format!("{mp}"), format!("{f} free of {t} — {pct}% used")]
        } else {
            vec![format!("{dev}"), format!("{f} free of {t} — {pct}% used")]
        }
    }

    /// True when (x, y) is over an entry cell (grid or list) in the current view.
    pub fn on_entry(&self, x: i32, y: i32) -> bool {
        if self.menu.is_some() || self.input_mode != crate::InputMode::None {
            return false;
        }
        layout::vis_index_at(self, x, y) >= 0
    }

    pub fn on_pointer_button(&mut self, x: i32, y: i32, time: u32, button: u32, pressed: bool) {
        if pressed {
            // any click dismisses the type-to-select indicator
            self.typeahead.clear();
        }
        if button == 272 {
            // BTN_LEFT
            if pressed {
                // Configure Toolbar dialog (modal)
                if self.toolbar_dlg.is_some() {
                    if self.toolbar_dlg_click(x, y) {
                        self.dirty = true;
                    }
                    return;
                }
                // modal input dialog (rename / new folder / new file / location …)
                if let Some(g) = layout::input_dlg_geom(self) {
                    if g.btn_ok.contains(x, y) {
                        self.commit_input();
                    } else if g.btn_cancel.contains(x, y) {
                        self.cancel_input();
                    } else if !g.field.contains(x, y) {
                        // click outside the dialog cancels it
                        self.cancel_input();
                    }
                    self.dirty = true;
                    return;
                }
                // properties dialog
                if self.props.is_some() {
                    if let Some(g) = layout::props_geom(self) {
                        if g.close.contains(x, y) {
                            self.props = None;
                        } else if let Some(t) = layout::props_tab_at(self, x, y) {
                            self.props_tab = t;
                        } else if !g.tabs.contains(x, y)
                            && !(g.x <= x && x < g.x + g.w && g.y <= y && y < g.y + g.h)
                        {
                            self.props = None;
                        }
                    }
                    self.dirty = true;
                    return;
                }
                if self.confirm_close {
                    if let Some(g) = layout::close_dlg_geom(self) {
                        if g.btn_cancel.contains(x, y) {
                            self.confirm_close = false;
                        } else if g.btn_tab.contains(x, y) {
                            self.confirm_close = false;
                            self.close_tab();
                        } else if g.btn_window.contains(x, y) {
                            self.running = false;
                        } else if g.checkbox.contains(x, y) {
                            self.confirm_dont_ask = !self.confirm_dont_ask;
                            if self.confirm_dont_ask {
                                self.cfg.confirm_close = false;
                                if let Some(p) = self.cfg_path.clone() {
                                    crate::config::set_keys(&p, &[("confirm_close", "false".to_string())]);
                                }
                            }
                        }
                    }
                    self.dirty = true;
                    return;
                }
                if self.confirm_import {
                    if let Some(g) = layout::import_dlg_geom(self) {
                        if g.btn_cancel.contains(x, y) {
                            self.confirm_import = false;
                            self.import_pending.clear();
                        } else if g.btn_ok.contains(x, y) {
                            self.do_import_actions();
                        }
                    }
                    self.dirty = true;
                    return;
                }
                if self.confirm_mkdir.is_some() {
                    if let Some(g) = layout::mkdir_dlg_geom(self) {
                        if g.btn_cancel.contains(x, y) {
                            self.confirm_mkdir = None;
                        } else if g.btn_create.contains(x, y) {
                            self.do_create_url_folder();
                        }
                    }
                    self.dirty = true;
                    return;
                }
                if self.menu.is_some() {
                    // resolve click target (submenu first, it floats on top)
                    let target = self.menu_click_at(x, y);
                    if let Some((is_sub, i, si)) = target {
                        if is_sub {
                            self.activate_sub_index(i, si);
                        } else {
                            self.activate_menu_index(i);
                        }
                        return;
                    }
                    // click outside closes
                    self.menu = None;
                    self.dirty = true;
                    // fall through so the click still acts
                }
                // Inline URL bar: click sets caret / starts a drag selection,
                // the red X closes, a click elsewhere closes and falls through
                // so the click still acts.
                if self.input_mode == InputMode::Url {
                    if layout::url_drop_at(self, x, y) {
                        self.open_url_dropdown();
                        self.dirty = true;
                        return;
                    }
                    if layout::url_go_at(self, x, y) {
                        self.commit_input();
                        return;
                    }
                    if layout::search_close_at(self, x, y) {
                        self.cancel_url_edit();
                        self.dirty = true;
                        return;
                    }
                    if layout::url_field_at(self, x, y) {
                        self.input_cursor = layout::url_caret_at(self, x);
                        self.url_sel = Some((self.input_cursor, self.input_cursor));
                        self.url_drag_sel = true;
                        self.dirty = true;
                        return;
                    }
                    self.cancel_url_edit();
                    self.dirty = true;
                }
                if let Some(ri) = layout::menu_bar_at(self, x, y) {
                    // Click same open ribbon item toggles closed; other items switch.
                    if self.menu.as_ref().map(|m| !m.is_context && self.ribbon_which == ri as i32).unwrap_or(false) {
                        self.menu = None;
                        self.ribbon_which = -1;
                        self.dirty = true;
                    } else {
                        self.open_ribbon(ri);
                    }
                    return;
                }
                if let Some(i) = layout::tab_close_at(self, x, y) {
                    self.close_tab_at(i);
                    return;
                }
                if let Some(i) = layout::tab_at(self, x, y) {
                    self.select_tab(i);
                    self.dirty = true;
                    return;
                }
                let nav = layout::nav_at(self, x, y);
                if nav >= 0 {
                    self.nav_action(nav);
                    return;
                }
                if layout::search_close_at(self, x, y) {
                    self.close_search();
                    return;
                }
                // generic toolbar item dispatch (nav, split, search, views, custom)
                if let Some((_, it)) = layout::toolbar_at(self, x, y) {
                    match &it {
                        crate::toolbar::ToolbarItem::Back => self.nav_action(0),
                        crate::toolbar::ToolbarItem::Forward => self.nav_action(1),
                        crate::toolbar::ToolbarItem::Up => self.nav_action(2),
                        crate::toolbar::ToolbarItem::Home => self.nav_action(3),
                        crate::toolbar::ToolbarItem::Split => self.toggle_split(),
                        crate::toolbar::ToolbarItem::Search => self.begin_search(),
                        crate::toolbar::ToolbarItem::List
                        | crate::toolbar::ToolbarItem::Grid
                        | crate::toolbar::ToolbarItem::Compact => {
                            let vm = match &it {
                                crate::toolbar::ToolbarItem::Grid => 1,
                                crate::toolbar::ToolbarItem::Compact => 2,
                                _ => 0,
                            };
                            self.tab_mut().view = match vm {
                                1 => crate::tab::ViewMode::Grid,
                                2 => crate::tab::ViewMode::Compact,
                                _ => crate::tab::ViewMode::List,
                            };
                            self.dirty = true;
                            self.request_visible_thumbs();
                        }
                        crate::toolbar::ToolbarItem::Custom(name) => {
                            self.run_toolbar_action(name.clone());
                        }
                        crate::toolbar::ToolbarItem::Location
                        | crate::toolbar::ToolbarItem::Separator => {}
                    }
                    return;
                }
                if layout::location_tray_at(self, x, y) && layout::path_seg_at(self, x, y).is_none() {
                    self.begin_url_edit();
                    return;
                }
                // pane resize grips (before row clicks)
                if layout::sidebar_splitter_at(self, x, y) {
                    self.side_resize = true;
                    self.preview_resize = false;
                    self.dirty = true;
                    return;
                }
                if layout::preview_splitter_at(self, x, y) {
                    self.preview_resize = true;
                    self.side_resize = false;
                    self.dirty = true;
                    return;
                }
                if layout::split_splitter_at(self, x, y) {
                    self.split_resize = true;
                    self.side_resize = false;
                    self.preview_resize = false;
                    self.dirty = true;
                    return;
                }
                // sidebar scrollbar: thumb drag / track page jump
                if let Some((sx, sy, sh)) = layout::sidebar_scrollbar_geom(self) {
                    if x >= sx - 2 && x < sx + 10 {
                        if y >= sy && y < sy + sh {
                            self.side_sdrag = true;
                            self.side_sdrag_off = y - sy;
                        } else if y >= layout::bar_h(self) {
                            let max = layout::side_scroll_max(self);
                            let page = layout::visible_rows(self).max(1);
                            self.side_scroll = if y < sy {
                                self.side_scroll.saturating_sub(page)
                            } else {
                                (self.side_scroll + page).min(max)
                            };
                        }
                        self.dirty = true;
                        return;
                    }
                }
                let srow = layout::sidebar_at(self, x, y);
                if srow >= 0 {
                    self.sidebar_nav(srow);
                    return;
                }
                // Split mode: presses inside the right pane act on the pane
                // and move focus there.
                if layout::split_active(self) && layout::pane_of(self, x, y) == 1 {
                    self.press_content_pane(x, y, time, 1);
                    self.focus_right_pane();
                    return;
                }
                if let Some(k) = layout::header_key_at(self, x, y) {
                    let t = self.tab_mut();
                    if t.sort_key == k {
                        t.sort_desc = !t.sort_desc;
                    } else {
                        t.sort_key = k;
                        t.sort_desc = false;
                    }
                    self.apply_sort();
                    self.save_state();
                    return;
                }
                if let Some(p) = layout::path_seg_at(self, x, y) {
                    if p != self.tab().cwd {
                        self.cd(p);
                    }
                    return;
                }
                if let Some((sx, sy, sh)) = layout::scrollbar_geom(self) {
                    // wider hit target (matches 8–10px track)
                    if x >= sx - 2 && x < sx + 10 {
                        if y >= sy && y < sy + sh {
                            // thumb drag
                            self.scroll_drag = true;
                            self.scroll_drag_pane = 0;
                            self.scroll_drag_off = y - sy;
                        } else {
                            // click track: page jump toward click
                            let page = layout::visible_units(self).max(1);
                            let max = layout::max_scroll(self);
                            let cur = self.tab().scroll;
                            let next = if y < sy {
                                cur.saturating_sub(page)
                            } else {
                                (cur + page).min(max)
                            };
                            self.tab_mut().scroll = next;
                        }
                        self.dirty = true;
                        return;
                    }
                }
                if layout::list_empty_at(self, x, y) {
                    // Rubberband on empty area of the list viewport.
                    self.start_rubber(x, y, 0);
                    self.last_click_time = 0;
                    return;
                }
                // Click on empty space anywhere else (sidebar gaps/headers,
                // preview pane, status/path bars, …) clears the selection;
                // Ctrl keeps it intact so a drag can still add to it.
                if !self.ctrl && layout::vis_index_at(self, x, y) < 0 {
                    self.deselect_all();
                }
                self.on_select_click(x, y, time, 0);
            } else {
                self.url_drag_sel = false;
                if self.rubber.is_some() {
                    self.end_rubber();
                }
                if self.scroll_drag {
                    self.scroll_drag = false;
                    self.dirty = true;
                }
                if self.side_sdrag {
                    self.side_sdrag = false;
                    self.dirty = true;
                }
                if self.side_resize {
                    self.side_resize = false;
                    self.dirty = true;
                    // remember the dragged pane width across restarts
                    self.save_state();
                }
                if self.preview_resize {
                    self.preview_resize = false;
                    self.dirty = true;
                    self.save_state();
                }
                if self.split_resize {
                    self.split_resize = false;
                    self.dirty = true;
                    self.save_state();
                }
            }
            self.dirty = true;
        } else if button == 274 {
            // BTN_MIDDLE: close the tab under the pointer
            if pressed {
                if self.menu.is_some() {
                    let target = self.menu_click_at(x, y);
                    if let Some((is_sub, i, si)) = target {
                        if is_sub {
                            self.activate_sub_index(i, si);
                        } else {
                            self.activate_menu_index(i);
                        }
                        return;
                    }
                    // click outside closes
                    self.menu = None;
                    self.dirty = true;
                    // fall through so the click still acts
                }
                if let Some(ri) = layout::menu_bar_at(self, x, y) {
                    self.open_ribbon(ri);
                    return;
                }
                if let Some(i) = layout::tab_at(self, x, y) {
                    self.close_tab_at(i);
                }
            }
        } else if button == 273 {
            // BTN_RIGHT: URL bar gets its edit menu; sidebar rows get their
            // own menu (mount/unmount/open/properties); everywhere else the
            // file context menu.
            if pressed {
                if self.input_mode == InputMode::Url && layout::location_tray_at(self, x, y) {
                    self.open_url_menu(x, y);
                } else if layout::sidebar_at(self, x, y) >= 0 {
                    self.open_sidebar_menu(x, y);
                } else {
                    self.open_context_menu(x, y);
                }
            }
        }
    }

    /// Left press inside the right split pane's content region (rows,
    /// headers, scrollbar, rubberband). Operates on the embedded pane.
    fn press_content_pane(&mut self, x: i32, y: i32, time: u32, pane: i32) {
        let (x0, w) = layout::pane_geom(self, pane);
        let t = self.pane_tab(pane);
        if let Some(k) = layout::header_key_at_for(self, t, x0, w, x, y) {
            {
                let tt = self.pane_tab_mut(pane);
                if tt.sort_key == k {
                    tt.sort_desc = !tt.sort_desc;
                } else {
                    tt.sort_key = k;
                    tt.sort_desc = false;
                }
            }
            let tt = self.pane_tab_mut(pane);
            crate::fs::sort_entries(&mut tt.entries, true, tt.sort_key, tt.sort_desc);
            tt.rebuild_visible();
            self.dirty = true;
            return;
        }
        if let Some((sx, sy, sh)) = layout::scrollbar_geom_for(self, t, x0, w) {
            if x >= sx - 2 && x < sx + 10 {
                if y >= sy && y < sy + sh {
                    self.scroll_drag = true;
                    self.scroll_drag_pane = 1;
                    self.scroll_drag_off = y - sy;
                } else {
                    let page = layout::scroll_units(self, self.pane_tab(pane), w).0.max(1);
                    let max = layout::max_scroll_for(self, t, x0, w);
                    let cur = t.scroll;
                    let next = if y < sy {
                        cur.saturating_sub(page)
                    } else {
                        (cur + page).min(max)
                    };
                    self.pane_tab_mut(pane).scroll = next;
                }
                self.dirty = true;
                return;
            }
        }
        if layout::list_empty_at_for(self, t, x0, w, x, y) {
            self.start_rubber(x, y, pane);
            self.last_click_time = 0;
            return;
        }
        if !self.ctrl && layout::vis_index_at_for(self, t, x0, w, x, y) < 0 {
            self.deselect_all();
        }
        self.on_select_click(x, y, time, pane);
    }

    /// Click-select in a pane (0 = focused/left, 1 = right split pane). A
    /// plain click records the range anchor; Shift+click selects the span
    /// between the anchor and the clicked row.
    fn on_select_click(&mut self, x: i32, y: i32, time: u32, pane: i32) {
        let anchor_now = self.range_anchor;
        let mut set_anchor: Option<i32> = None;
        let (x0, w) = layout::pane_geom(self, pane);
        let idx = layout::vis_index_at_for(self, self.pane_tab(pane), x0, w, x, y);
        if idx >= 0 {
            self.pane_tab_mut(pane).sel_none = false;
            let ctrl = self.ctrl;
            let shift = self.shift;
            let mut do_range: Option<(usize, usize)> = None;
            {
                let t = self.pane_tab_mut(pane);
                let vi = idx as usize;
                if vi < t.nvis() {
                    if ctrl {
                        let ei = t.vis[vi];
                        if let Some(e) = t.entries.get_mut(ei) {
                            e.selected = !e.selected;
                        }
                        t.sel = vi;
                    } else if shift {
                        t.sel = vi;
                        let anchor = if anchor_now >= 0 {
                            anchor_now as usize
                        } else {
                            vi
                        };
                        do_range = Some((anchor, vi));
                    } else {
                        for i in &t.vis {
                            if let Some(e) = t.entries.get_mut(*i) {
                                e.selected = false;
                            }
                        }
                        t.sel = vi;
                        if let Some(e) = t.entries.get_mut(t.vis[vi]) {
                            e.selected = true;
                        }
                        set_anchor = Some(vi as i32);
                    }
                }
            }
            if let Some((anchor, target)) = do_range {
                self.select_range(pane, anchor, target);
            }
            if let Some(a) = set_anchor {
                self.range_anchor = a;
            }
            self.count_selected();
        }
        // double-click → open (focus the right pane first so open acts on it)
        if time > 0
            && self.last_click_time > 0
            && time.wrapping_sub(self.last_click_time) < 400
            && (x - self.last_click_x).abs() < 8
            && (y - self.last_click_y).abs() < 8
        {
            if pane == 1 {
                self.focus_right_pane();
            }
            self.open_selected();
            self.last_click_time = 0;
        } else {
            self.last_click_time = time;
            self.last_click_x = x;
            self.last_click_y = y;
        }
    }

    /// Select the span [anchor..=target] in a pane (Shift+click range).
    pub fn select_range(&mut self, pane: i32, anchor: usize, target: usize) {
        let t = self.pane_tab_mut(pane);
        if t.vis.is_empty() {
            return;
        }
        let a = anchor.min(target);
        let b = anchor.max(target);
        for (i, vi) in t.vis.iter().enumerate() {
            if let Some(e) = t.entries.get_mut(*vi) {
                e.selected = i >= a && i <= b;
            }
        }
        self.count_selected();
        self.dirty = true;
    }

    pub fn scroll_drag_to(&mut self, y: i32) {
        let pane = self.scroll_drag_pane;
        let (x0, w) = layout::pane_geom(self, pane);
        layout::scroll_drag_for(self, pane, x0, w, y, self.scroll_drag_off);
        self.dirty = true;
    }

    /// Wheel scroll — routed to the pane under the pointer when split, or to
    /// the sidebar when the pointer is over it.
    pub fn on_scroll(&mut self, dy: i32) {
        if dy == 0 {
            return;
        }
        if self.toolbar_dlg.is_some() {
            if let Some(dlg) = self.toolbar_dlg.as_mut() {
                let visible = 8.min(dlg.items.len().max(1));
                let max_scroll = dlg.items.len() as i32 - visible as i32;
                let mut s = dlg.scroll + dy;
                if s < 0 {
                    s = 0;
                }
                if s > max_scroll {
                    s = max_scroll;
                }
                if s != dlg.scroll {
                    dlg.scroll = s;
                    self.dirty = true;
                }
            }
            return;
        }
        if layout::sidebar_area_at(self, self.ptr_x, self.ptr_y) {
            self.scroll_sidebar(dy);
            return;
        }
        let pane = if layout::split_active(self) {
            layout::pane_of(self, self.ptr_x, self.ptr_y).max(0)
        } else {
            0
        };
        self.on_scroll_pane(dy, pane);
    }

    /// Scroll the sidebar by `dy` wheel steps (dy > 0 = down).
    fn scroll_sidebar(&mut self, dy: i32) {
        let max = layout::side_scroll_max(self);
        if max == 0 {
            return;
        }
        let step = 3usize;
        let mut s = self.side_scroll as i32 + dy * step as i32;
        if s < 0 {
            s = 0;
        }
        if s > max as i32 {
            s = max as i32;
        }
        if s as usize != self.side_scroll {
            self.side_scroll = s as usize;
            self.dirty = true;
        }
    }

    pub fn on_scroll_pane(&mut self, dy: i32, pane: i32) {
        if dy == 0 {
            return;
        }
        // dy > 0 = wheel down / content moves up → increase scroll index
        let (_, w) = layout::pane_geom(self, pane);
        let step = match self.pane_tab(pane).view {
            crate::tab::ViewMode::Grid => layout::grid_cols_in(self, w).max(1) as i32,
            _ => 3,
        };
        let max = layout::max_scroll_for(self, self.pane_tab(pane), 0, w) as i32;
        let changed = {
            let t = self.pane_tab_mut(pane);
            let mut s = t.scroll as i32;
            if dy > 0 {
                s += step * dy.max(1);
            } else {
                s += step * dy; // dy negative
            }
            if s < 0 {
                s = 0;
            }
            if s > max {
                s = max;
            }
            if s as usize != t.scroll {
                t.scroll = s as usize;
                true
            } else {
                false
            }
        };
        if changed {
            self.dirty = true;
        }
        if self.pane_tab(pane).view == ViewMode::Grid {
            if pane == 0 {
                self.request_visible_thumbs();
            } else {
                self.request_visible_thumbs_pane(1);
            }
        }
    }


    /// Clamp scroll after listing/resize so the thumb never hangs past the end.
    pub fn clamp_scroll(&mut self) {
        let max = layout::max_scroll(self);
        let t = self.tab_mut();
        if t.scroll > max {
            t.scroll = max;
            self.dirty = true;
        }
    }

    // ------------------------------------------------------------------
    // rubberband selection
    // ------------------------------------------------------------------

    /// Clear every selected flag (keeps the `sel` index; `sel_none` makes
    /// nothing render as active). Used by empty-space clicks.
    pub fn deselect_all(&mut self) {
        let t = self.tab_mut();
        t.sel_none = true;
        for e in t.entries.iter_mut() {
            e.selected = false;
        }
        self.count_selected();
        self.dirty = true;
    }

    /// Begin rubberband on empty-area left press (in the list viewport).
    pub fn start_rubber(&mut self, x: i32, y: i32, pane: i32) {
        self.rubber = Some((x, y, x, y));
        self.rubber_additive = self.ctrl;
        self.rubber_pane = pane;
        if !self.rubber_additive {
            let t = self.pane_tab_mut(pane);
            for e in t.entries.iter_mut() {
                e.selected = false;
            }
            self.count_selected();
        }
        self.dirty = true;
    }

    /// Extend the rubberband to (x, y) and update selection.
    pub fn update_rubber(&mut self, x: i32, y: i32) {
        let Some((sx, sy, _, _)) = self.rubber else { return };
        self.rubber = Some((sx, sy, x, y));
        let pane = self.rubber_pane;
        let (x0, y0) = (sx.min(x), sy.min(y));
        let (x1, y1) = (sx.max(x), sy.max(y));
        let additive = self.rubber_additive;
        let (px0, pw) = layout::pane_geom(self, pane);
        let mut set: Vec<(usize, bool)> = Vec::new();
        {
            let t = self.pane_tab(pane);
            for vi in 0..t.nvis() {
                if let Some((ex0, ey0, ew, eh)) = layout::entry_rect_for(self, t, px0, pw, vi) {
                    let inside = x0 < ex0 + ew && ex0 < x1 && y0 < ey0 + eh && ey0 < y1;
                    set.push((t.vis[vi], inside));
                }
            }
        }
        let t = self.pane_tab_mut(pane);
        for (ei, inside) in set {
            if let Some(e) = t.entries.get_mut(ei) {
                if inside {
                    e.selected = true;
                } else if !additive {
                    e.selected = false;
                }
            }
        }
        self.count_selected();
        self.dirty = true;
    }

    /// Finish a rubberband selection (button release).
    pub fn end_rubber(&mut self) {
        self.rubber = None;
        self.dirty = true;
    }

    // ------------------------------------------------------------------
    // drag & drop
    // ------------------------------------------------------------------

    /// Paths to drag out for a DnD source.
    pub fn dnd_paths(&self) -> Vec<PathBuf> {
        self.selected_paths()
    }

    /// Update the drop target (dir row under pointer or the cwd fallback).
    pub fn dnd_update_target(&mut self, x: i32, y: i32) {
        let (dir, row) = {
            let t = self.tab();
            let idx = layout::vis_index_at(self, x, y);
            if idx >= 0 {
                let vi = idx as usize;
                if vi < t.nvis() {
                    let e = &t.entries[t.vis[vi]];
                    if e.is_dir {
                        (Some(t.entry_path(e)), idx)
                    } else {
                        (None, -1)
                    }
                } else {
                    (None, -1)
                }
            } else {
                (None, -1)
            }
        };
        self.dnd_drop_dir = dir.or_else(|| Some(self.tab().cwd.clone()));
        if row >= 0 {
            self.dnd_hover_row = row;
        }
        self.dirty = true;
    }

    /// Dolphin-style popup on drop: Copy / Move / link / new-folder ops.
    pub fn open_dnd_menu(&mut self, x: i32, y: i32) {
        let items = vec![
            Menu::item(MenuId::DndCopy, "Copy Here", true),
            Menu::item(MenuId::DndMove, "Move Here", true),
            Menu::sep(),
            Menu::item(MenuId::DndMoveNew, "Move to New Folder…", true),
            Menu::item(MenuId::DndCopyNew, "Copy to New Folder…", true),
            Menu::sep(),
            Menu::item(MenuId::DndLink, "Link Here (Absolute)", true),
            Menu::item(MenuId::DndLinkRel, "Link Here (Relative)", true),
            Menu::item(MenuId::DndHardlink, "Create Hardlink", true),
            Menu::sep(),
            Menu::item(MenuId::DndCancel, "Cancel", true),
        ];
        let mut menu = Menu {
            x,
            y,
            items,
            hover: 0,
            is_context: true,
            sub_open: None,
        };
        self.clamp_menu(&mut menu);
        self.menu = Some(menu);
        self.dirty = true;
    }

    /// Start the "move/copy to new folder" prompt. mode: 0 = move, 1 = copy.
    fn begin_dnd_new(&mut self, mode: i32) {
        self.dnd_new_mode = mode;
        self.begin_input(InputMode::DndNewDir, "new folder name:", "New Folder");
    }

    /// Execute a completed drop. mode: 0 copy, 1 move, 2 symlink (absolute),
    /// 3 symlink (relative), 4 hardlink.
    pub fn dnd_execute(&mut self, mode: i32) {
        let dir = self.dnd_drop_dir.clone().unwrap_or_else(|| self.tab().cwd.clone());
        self.dnd_execute_into(dir, mode);
    }

    /// Like `dnd_execute` but drops into an explicit directory (used by the
    /// "to new folder" flow after the folder is created).
    fn dnd_execute_into(&mut self, dir: PathBuf, mode: i32) {
        let mut paths = std::mem::take(&mut self.dnd_uri_paths);
        if paths.is_empty() {
            // internal same-window drag: the source is the current selection
            paths = self.selected_paths();
        }
        let mut ok = 0usize;
        let mut fail = 0usize;
        for src in &paths {
            let name = src
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "item".into());
            let mut dst = dir.join(&name);
            if dst.exists() {
                dst = fs::unique_copy_name(&dir, &name);
            }
            let r = match mode {
                0 => fs::copy_path(src, &dst),
                1 => fs::move_path(src, &dst),
                2 => {
                    let _ = std::fs::remove_file(&dst);
                    fs::symlink_path(src, &dst)
                }
                3 => {
                    let _ = std::fs::remove_file(&dst);
                    let rel = fs::relative_path(&dir, src);
                    fs::symlink_path(&rel, &dst)
                }
                _ => {
                    let _ = std::fs::remove_file(&dst);
                    fs::hardlink_path(src, &dst)
                }
            };
            if r.is_ok() {
                ok += 1;
                match mode {
                    0 => self.undo_push(UndoOp::Copy { src: src.clone(), dst: dst.clone() }),
                    1 => self.undo_push(UndoOp::Move { src: src.clone(), dst: dst.clone() }),
                    _ => {}
                }
            } else {
                fail += 1;
            }
        }
        self.dnd_clear();
        self.reload();
        let verb = match mode {
            0 => "copied",
            1 => "moved",
            2 => "linked",
            3 => "linked (relative)",
            _ => "hardlinked",
        };
        if fail > 0 {
            self.set_status(format!("{verb} {ok}, {fail} failed"));
        } else {
            self.set_status(format!("{verb} {ok} item(s)"));
        }
    }

    /// Clear all drop/drag transient state.
    pub fn dnd_clear(&mut self) {
        self.dnd_active = false;
        self.dnd_hover = false;
        self.dnd_hover_row = -1;
        self.dnd_side_hover = false;
        self.dnd_drop_dir = None;
        self.dnd_uri_paths.clear();
        self.dnd_x = 0;
        self.dnd_y = 0;
        self.dirty = true;
    }

    pub fn selected_paths(&self) -> Vec<PathBuf> {
        let t = self.tab();
        let mut out = Vec::new();
        for e in &t.entries {
            if e.selected {
                out.push(t.entry_path(e));
            }
        }
        if out.is_empty() {
            if let Some(e) = t.selected() {
                out.push(t.entry_path(e));
            }
        }
        out
    }

    pub fn clip_copy(&mut self) {
        let paths = self.selected_paths();
        if paths.is_empty() {
            return;
        }
        self.clip_paths = paths;
        self.clip_cut = false;
        let text = self
            .clip_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        fs::clipboard_set_text(&text);
        self.set_status(format!("copied {} item(s)", self.clip_paths.len()));
    }

    pub fn clip_cut(&mut self) {
        let paths = self.selected_paths();
        if paths.is_empty() {
            return;
        }
        self.clip_paths = paths;
        self.clip_cut = true;
        let text = self
            .clip_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        fs::clipboard_set_text(&text);
        self.set_status(format!("cut {} item(s)", self.clip_paths.len()));
    }

    pub fn clip_paste(&mut self) {
        if self.clip_paths.is_empty() {
            self.set_status("clipboard empty".into());
            return;
        }
        let dest_dir = self.tab().cwd.clone();
        let cut = self.clip_cut;
        let paths = self.clip_paths.clone();
        let mut ok = 0usize;
        let mut undo = Vec::new();
        for src in &paths {
            let name = src.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "item".into());
            let mut dst = dest_dir.join(&name);
            if dst.exists() {
                dst = fs::unique_copy_name(&dest_dir, &name);
            }
            let r = if cut {
                fs::move_path(src, &dst)
            } else {
                fs::copy_path(src, &dst)
            };
            if r.is_ok() {
                ok += 1;
                if cut {
                    undo.push(UndoOp::Move { src: src.clone(), dst: dst.clone() });
                } else {
                    undo.push(UndoOp::Copy { src: src.clone(), dst: dst.clone() });
                }
            }
        }
        for op in undo {
            self.undo_push(op);
        }
        if cut {
            self.clip_paths.clear();
            self.clip_cut = false;
        }
        self.reload();
        self.set_status(format!("pasted {ok} item(s)"));
    }

    pub fn duplicate_selected(&mut self) {
        let paths = self.selected_paths();
        if paths.is_empty() {
            return;
        }
        let mut ok = 0usize;
        for src in &paths {
            let parent = src.parent().unwrap_or_else(|| std::path::Path::new("/"));
            let name = src.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let dst = fs::unique_copy_name(parent, &name);
            if fs::copy_path(src, &dst).is_ok() {
                ok += 1;
                self.undo_push(UndoOp::Copy { src: src.clone(), dst });
            }
        }
        self.reload();
        self.set_status(format!("duplicated {ok}"));
    }

    /// Push a reversible file operation onto the undo stack (bounded).
    fn undo_push(&mut self, op: UndoOp) {
        self.undo_stack.push(op);
        if self.undo_stack.len() > 64 {
            self.undo_stack.remove(0);
        }
    }

    /// Undo the last copy or move (Ctrl+Z). A copy deletes the destination;
    /// a move restores the original path.
    fn undo_last(&mut self) {
        let Some(op) = self.undo_stack.pop() else {
            self.set_status("nothing to undo".into());
            return;
        };
        let r = match &op {
            UndoOp::Copy { dst, .. } => fs::delete_path(dst),
            UndoOp::Move { src, dst } => {
                if dst.exists() {
                    fs::move_path(dst, src)
                } else {
                    Ok(())
                }
            }
        };
        match r {
            Ok(_) => {
                self.reload();
                self.set_status(match &op {
                    UndoOp::Copy { src, dst } => {
                        format!("undid copy of {} → {}", src.display(), dst.display())
                    }
                    UndoOp::Move { src, .. } => format!("undid move of {}", src.display()),
                });
            }
            Err(_) => {
                self.set_err(format!(
                    "undo failed ({})",
                    match &op {
                        UndoOp::Copy { dst, .. } => dst.display().to_string(),
                        UndoOp::Move { src, .. } => src.display().to_string(),
                    }
                ));
                // keep the failed op around so the user can retry after fixing
                // the cause (e.g. name clash), like Dolphin.
                self.undo_stack.push(op);
            }
        }
    }

    pub fn delete_selected(&mut self) {
        let paths = self.selected_paths();
        if paths.is_empty() {
            return;
        }
        let mut ok = 0usize;
        for p in &paths {
            if fs::delete_path(p).is_ok() {
                ok += 1;
            }
        }
        self.reload();
        self.set_status(format!("deleted {ok}"));
    }

    pub fn open_terminal_here(&mut self) {
        let mut term = self.cfg.terminal.clone();
        if term.is_empty() {
            term = fs::probe_terminal();
        }
        if term.is_empty() {
            self.set_err("no terminal found".into());
            return;
        }
        let cwd = self.tab().cwd.clone();
        // common CLIs: -e is not always needed; just spawn with cwd
        let r = std::process::Command::new(&term).current_dir(&cwd).spawn();
        if r.is_err() {
            self.set_err(format!("failed to launch {term}"));
        } else {
            self.set_status(format!("terminal: {term}"));
        }
    }

    pub fn copy_path_text(&mut self, kind: MenuId) {
        let paths = self.selected_paths();
        if paths.is_empty() {
            return;
        }
        let mut parts = Vec::new();
        for p in &paths {
            let s = match kind {
                MenuId::CopyFullPath => p.display().to_string(),
                MenuId::CopyBaseName => p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                MenuId::CopyParent => p
                    .parent()
                    .map(|n| n.display().to_string())
                    .unwrap_or_else(|| "/".into()),
                MenuId::CopyStem => p
                    .file_stem()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                _ => continue,
            };
            parts.push(s);
        }
        let text = parts.join("\n");
        fs::clipboard_set_text(&text);
        self.set_status("copied to clipboard".into());
    }

    pub fn go_link_target(&mut self) {
        let Some(e) = self.tab().selected().cloned() else { return };
        if !e.is_link {
            return;
        }
        let full = fs::full_path(&self.tab().cwd, &e.name);
        let Ok(target) = std::fs::read_link(&full) else {
            self.set_err("broken symlink".into());
            return;
        };
        let resolved = if target.is_absolute() {
            target
        } else {
            self.tab().cwd.join(target)
        };
        // Open the target in a new window, keeping the current view intact.
        if resolved.is_dir() {
            self.spawn_new_window(Some(resolved));
        } else if let Some(parent) = resolved.parent() {
            self.spawn_new_window(Some(parent.to_path_buf()));
        }
    }

    /// Right-click menu for the inline URL bar (cut/copy/paste/undo).
    pub fn open_url_menu(&mut self, x: i32, y: i32) {
        let (lo, hi) = self.url_range();
        let has_sel = hi > lo;
        let items = vec![
            Menu::item(MenuId::UrlUndo, "Undo\tCtrl+Z", !self.url_undo.is_empty()),
            Menu::sep(),
            Menu::item(MenuId::UrlCut, "Cut\tCtrl+X", has_sel),
            Menu::item(MenuId::UrlCopy, "Copy\tCtrl+C", has_sel),
            Menu::item(MenuId::UrlPaste, "Paste\tCtrl+V", true),
            Menu::sep(),
            Menu::item(MenuId::UrlClear, "Clear All", !self.input.is_empty()),
            Menu::item(MenuId::UrlSelectAll, "Select All\tCtrl+A", !self.input.is_empty()),
        ];
        let mut menu = Menu {
            x,
            y,
            items,
            hover: 0,
            is_context: true,
            sub_open: None,
        };
        self.clamp_menu(&mut menu);
        self.menu = Some(menu);
        self.dirty = true;
    }

    #[allow(clippy::vec_init_then_push)]
    pub fn open_context_menu(&mut self, x: i32, y: i32) {
        // select row under cursor if any
        let idx = layout::vis_index_at(self, x, y);
        let ctrl = self.ctrl;
        if idx >= 0 {
            let t = self.tab_mut();
            let vi = idx as usize;
            if vi < t.nvis() {
                t.sel_none = false;
                let already = t.entries.get(t.vis[vi]).map(|e| e.selected).unwrap_or(false);
                if !already && !ctrl {
                    for i in t.vis.clone() {
                        if let Some(e) = t.entries.get_mut(i) {
                            e.selected = false;
                        }
                    }
                    t.sel = vi;
                    if let Some(e) = t.entries.get_mut(t.vis[vi]) {
                        e.selected = true;
                    }
                } else {
                    t.sel = vi;
                }
            }
            self.count_selected();
        }

        let has_sel = !self.selected_paths().is_empty();
        let is_link = self.tab().selected().map(|e| e.is_link).unwrap_or(false);
        let cwd = self.tab().cwd.clone();
        let bookmarked = self.bookmark_paths.iter().any(|p| p == &cwd);
        // Only directory selections can be added as sidebar shortcuts (a
        // symlink counts when it resolves to a directory).
        let sel_all_dirs = self
            .tab()
            .entries
            .iter()
            .filter(|e| e.selected)
            .all(|e| e.is_dir);
        // "Open With…" applies to a single non-directory selection
        let open_with_ok = self
            .tab()
            .selected()
            .map(|e| !e.is_dir)
            .unwrap_or(false);
        let mut items = Vec::new();
        items.push(Menu::item(MenuId::Undo, "Undo copy/move\tCtrl+Z", !self.undo_stack.is_empty()));
        items.push(Menu::sep());
        items.push(Menu::item(MenuId::Open, "Open", has_sel));
        items.push(Menu::item(MenuId::OpenWith, "Open With…", open_with_ok));
        items.push(Menu::item(MenuId::OpenTerminal, "Open Terminal Here", true));
        items.push(Menu::sep());
        items.push(Menu::item(MenuId::Cut, "Cut", has_sel));
        items.push(Menu::item(MenuId::Copy, "Copy", has_sel));
        items.push(Menu::item(MenuId::Paste, "Paste", !self.clip_paths.is_empty()));
        items.push(Menu::item(MenuId::Duplicate, "Duplicate Here", has_sel));
        items.push(Menu::sep());
        items.push(Menu::item(MenuId::CopyFullPath, "Copy Full Path", has_sel));
        items.push(Menu::item(MenuId::CopyBaseName, "Copy Base Name", has_sel));
        items.push(Menu::item(MenuId::CopyStem, "Copy Name Without Extension", has_sel));
        items.push(Menu::item(MenuId::CopyParent, "Copy Parent Path", has_sel));
        if is_link {
            items.push(Menu::item(MenuId::GoToLinkTarget, "Go to Symlink Target", true));
        }
        items.push(Menu::sep());
        items.push(Menu::item(MenuId::Rename, "Rename…", has_sel));
        items.push(Menu::item(MenuId::Delete, "Delete", has_sel));
        items.push(Menu::item(MenuId::NewFolder, "New Folder…", true));
        items.push(Menu::item(MenuId::NewFile, "New File…", true));
        items.push(Menu::sep());
        if has_sel {
            items.push(Menu::item(MenuId::AddShortcut, "Add to Places", sel_all_dirs));
        } else if bookmarked {
            items.push(Menu::item(MenuId::RemoveShortcut, "Remove from Shortcuts", true));
        } else {
            items.push(Menu::item(MenuId::AddShortcut, "Add Current Folder to Places", true));
        }
        items.push(Menu::item(MenuId::Properties, "Properties", has_sel));

        // Dolphin-style Sort By submenu
        items.push(Menu::sep());
        items.push(self.sort_submenu_item());

        // Thunar-style custom actions (uca.xml) matching the current selection
        self.custom_actions.clear();
        if has_sel {
            let (cats, names): (Vec<_>, Vec<_>) = {
                let t = self.tab();
                t.entries
                    .iter()
                    .filter(|e| e.selected)
                    .map(|e| (crate::actions::file_category(e), e.name.clone()))
                    .unzip()
            };
            for a in crate::actions::load_custom_actions() {
                if crate::actions::action_applies(&a, &cats, &names) {
                    self.custom_actions.push((a.name.clone(), a.command.clone()));
                }
            }
            if !self.custom_actions.is_empty() {
                items.push(Menu::sep());
                for (i, (name, _)) in self.custom_actions.iter().enumerate() {
                    items.push(Menu::item(MenuId::CustomAction(i as i32), name.clone(), true));
                }
            }
        }

        let mut menu = Menu {
            x,
            y,
            items,
            hover: -1,
            is_context: true,
            sub_open: None,
        };
        self.clamp_menu(&mut menu);
        self.menu = Some(menu);
        self.dirty = true;
    }

    /// "Open With…" picker: apps for the selected file's mime type, plus
    /// Custom Command and Set as Default. Opens as a submenu at the origin of
    /// the context menu that spawned it.
    pub fn open_open_with(&mut self) {
        let Some(e) = self.tab().selected().cloned() else { return };
        if e.is_dir {
            return;
        }
        let full = fs::full_path(&self.tab().cwd, &e.name);
        let mime = fs::mime_for_path(&full);
        let apps = fs::apps_for_mime(&mime);
        self.open_with_list = apps;
        let default = fs::default_app_for(&mime);
        let mut items = Vec::new();
        if self.open_with_list.is_empty() {
            items.push(Menu::item(MenuId::OpenWithCustom, "(no apps found) — Custom Command…", true));
        } else {
            for (i, (name, _, _)) in self.open_with_list.iter().enumerate() {
                items.push(Menu::item(MenuId::OpenWithApp(i as i32), name.clone(), true));
            }
        }
        items.push(Menu::sep());
        items.push(Menu::item(MenuId::OpenWithCustom, "Custom Command…", true));
        items.push(Menu::item(MenuId::SetDefaultApp, "Set as Default for This File Type…", true));
        let mut menu = Menu {
            x: self.menu_x + self.cfg.padding / 2,
            y: self.menu_y,
            items,
            hover: -1,
            is_context: true,
            sub_open: None,
        };
        self.clamp_menu(&mut menu);
        self.menu = Some(menu);
        // surface the mime type in the status line so the choice is clear
        let cur = default.unwrap_or_else(|| self.cfg.opener.clone());
        self.set_status(format!("{mime} — default: {cur}"));
    }

    /// Context menu for a right-click on a sidebar row (volumes, places,
    /// bookmarks). Mounted volumes get Unmount / Open in New Tab / New
    /// Window / Split / Properties; unmounted devices get Mount; the root
    /// partition never gets an Unmount option.
    pub fn open_sidebar_menu(&mut self, x: i32, y: i32) {
        let row = layout::sidebar_at(self, x, y);
        if row < 0 {
            return;
        }
        self.ctx_path = None;
        self.ctx_place_key = None;
        let mut items = Vec::new();
        let kind = if row < 7 {
            layout::SideKind::Place(row as u8)
        } else if row >= 400 {
            layout::SideKind::Removable((row - 400) as usize)
        } else if row >= 300 {
            layout::SideKind::Bookmark((row - 300) as usize)
        } else if row >= 200 {
            layout::SideKind::Unmounted((row - 200) as usize)
        } else if row >= 100 {
            layout::SideKind::Network((row - 100) as usize)
        } else {
            layout::SideKind::Volume((row - 10) as usize)
        };
        match &kind {
            layout::SideKind::Volume(i) => {
                if *i >= self.side_n {
                    return;
                }
                let p = self.side_paths[*i].clone();
                self.ctx_path = Some(p);
                items.push(Menu::item(MenuId::Unmount, "Unmount", true));
                items.push(Menu::sep());
                items.push(Menu::item(MenuId::OpenNewTab, "Open in New Tab", true));
                items.push(Menu::item(MenuId::NewWindow, "Open in New Window", true));
                items.push(Menu::item(MenuId::OpenSplit, "Open in Split", true));
                items.push(Menu::sep());
                items.push(Menu::item(MenuId::Properties, "Properties", true));
            }
            layout::SideKind::Unmounted(i) => {
                if *i >= self.unmounted_n {
                    return;
                }
                let p = self.unmounted_paths[*i].clone();
                self.ctx_path = Some(p);
                items.push(Menu::item(MenuId::Mount, "Mount", true));
                items.push(Menu::sep());
                items.push(Menu::item(MenuId::Properties, "Properties", true));
            }
            layout::SideKind::Removable(i) => {
                if *i >= self.removable_n {
                    return;
                }
                let p = self.removable_paths[*i].clone();
                self.ctx_path = Some(p);
                if self.removable_mounted[*i] {
                    items.push(Menu::item(MenuId::Unmount, "Unmount", true));
                    items.push(Menu::sep());
                    items.push(Menu::item(MenuId::OpenNewTab, "Open in New Tab", true));
                    items.push(Menu::item(MenuId::NewWindow, "Open in New Window", true));
                    items.push(Menu::item(MenuId::OpenSplit, "Open in Split", true));
                    items.push(Menu::sep());
                    items.push(Menu::item(MenuId::Properties, "Properties", true));
                } else {
                    items.push(Menu::item(MenuId::Mount, "Mount", true));
                    items.push(Menu::sep());
                    items.push(Menu::item(MenuId::Properties, "Properties", true));
                }
            }
            layout::SideKind::Bookmark(i) => {
                if *i >= self.bookmark_n {
                    return;
                }
                let p = self.bookmark_paths[*i].clone();
                self.ctx_path = Some(p);
                items.push(Menu::item(MenuId::RemoveShortcut, "Remove from Shortcuts", true));
                items.push(Menu::sep());
                items.push(Menu::item(MenuId::OpenNewTab, "Open in New Tab", true));
                items.push(Menu::item(MenuId::NewWindow, "Open in New Window", true));
                items.push(Menu::item(MenuId::OpenSplit, "Open in Split", true));
                items.push(Menu::sep());
                items.push(Menu::item(MenuId::Properties, "Properties", true));
            }
            layout::SideKind::Place(p) => {
                let key = match *p {
                    0 => "home",
                    1 => "documents",
                    2 => "downloads",
                    3 => "music",
                    4 => "videos",
                    5 => "trash",
                    _ => "",
                };
                self.ctx_place_key = if key.is_empty() { None } else { Some(key.to_string()) };
                let path = if *p == 6 {
                    Some(PathBuf::from("/"))
                } else {
                    self.place_path(key)
                };
                if let Some(path) = path {
                    self.ctx_path = Some(path);
                    items.push(Menu::item(MenuId::OpenNewTab, "Open in New Tab", true));
                    items.push(Menu::item(MenuId::NewWindow, "Open in New Window", true));
                    items.push(Menu::item(MenuId::OpenSplit, "Open in Split", true));
                    items.push(Menu::sep());
                    items.push(Menu::item(MenuId::Properties, "Properties", true));
                    // default places can be hidden (restored from View → Hidden Places)
                    if !key.is_empty() {
                        let label = place_label(*p);
                        items.push(Menu::sep());
                        items.push(Menu::item(
                            MenuId::HidePlace,
                            format!("Hide {label} from Places"),
                            true,
                        ));
                    }
                }
            }
            _ => return,
        }
        let mut menu = Menu {
            x,
            y,
            items,
            hover: -1,
            is_context: true,
            sub_open: None,
        };
        self.clamp_menu(&mut menu);
        self.menu_x = x;
        self.menu_y = y;
        self.menu = Some(menu);
        self.dirty = true;
    }

    /// Open the View → Configure Toolbar… dialog.
    pub fn open_toolbar_dialog(&mut self) {
        let mut items = self.cfg.toolbar.clone();
        let mut hidden = self.cfg.toolbar_hidden.clone();
        // offer custom actions that aren't on the toolbar yet (unchecked)
        for (name, _) in &self.custom_actions {
            let key = format!("ca:{name}");
            if !crate::toolbar::contains_key(&items, &key) {
                items.push(crate::toolbar::ToolbarItem::Custom(name.clone()));
                hidden.push(key);
            }
        }
        self.toolbar_dlg = Some(ToolbarDlg { items, hidden, sel: 0, scroll: 0 });
        self.menu = None;
        self.dirty = true;
    }

    /// Close the dialog, writing the working copies to config + config file.
    fn apply_toolbar_dialog(&mut self) {
        let Some(dlg) = self.toolbar_dlg.take() else { return };
        self.cfg.toolbar = dlg.items.clone();
        self.cfg.toolbar_hidden = dlg.hidden.clone();
        if let Some(p) = self.cfg_path.clone() {
            let toolbar = crate::toolbar::toolbar_to_str(&dlg.items);
            let hidden = dlg.hidden.join(";");
            crate::config::set_keys(&p, &[("toolbar", toolbar), ("toolbar_hidden", hidden)]);
        }
        self.dirty = true;
    }

    /// Reset the working copies to the default toolbar.
    fn toolbar_reset_dialog(&mut self) {
        if let Some(dlg) = self.toolbar_dlg.as_mut() {
            dlg.items = crate::toolbar::default_toolbar();
            dlg.hidden.clear();
            dlg.sel = 0;
            dlg.scroll = 0;
            self.dirty = true;
        }
    }

    /// Handle a left-click inside the Configure Toolbar dialog.
    fn toolbar_dlg_click(&mut self, x: i32, y: i32) -> bool {
        let Some(g) = layout::toolbar_dlg_geom(self) else {
            self.toolbar_dlg = None;
            self.dirty = true;
            return true;
        };
        if !(g.x <= x && x < g.x + g.w && g.y <= y && y < g.y + g.h) {
            self.toolbar_dlg = None; // click outside cancels
            self.dirty = true;
            return true;
        }
        if g.btn_close.contains(x, y) {
            self.apply_toolbar_dialog();
            return true;
        }
        if g.btn_up.contains(x, y) {
            if let Some(dlg) = self.toolbar_dlg.as_mut() {
                dlg.move_sel(-1);
            }
            self.dirty = true;
            return true;
        }
        if g.btn_down.contains(x, y) {
            if let Some(dlg) = self.toolbar_dlg.as_mut() {
                dlg.move_sel(1);
            }
            self.dirty = true;
            return true;
        }
        if g.btn_reset.contains(x, y) {
            self.toolbar_reset_dialog();
            return true;
        }
        if let Some(i) = layout::toolbar_dlg_row_at(self, x, y) {
            let toggle = layout::toolbar_dlg_check_at(self, x, y);
            if let Some(dlg) = self.toolbar_dlg.as_mut() {
                dlg.sel = i;
                if toggle {
                    dlg.toggle(i);
                }
            }
            self.dirty = true;
            return true;
        }
        true
    }

    /// Add the given paths as sidebar shortcuts (DnD drop onto the Places
    /// pane). Only directories — or symlinks resolving to a directory — are
    /// accepted; files, file-symlinks and broken symlinks are skipped.
    pub fn add_shortcuts(&mut self, paths: Vec<PathBuf>) {
        let mut added = 0usize;
        let mut skipped = 0usize;
        for p in paths {
            // `Path::is_dir` follows symlinks, so a symlink pointing at a
            // valid directory passes and a broken/file symlink is rejected.
            if !p.is_dir() {
                skipped += 1;
                continue;
            }
            if self.bookmark_paths.iter().any(|b| b == &p) {
                skipped += 1;
                continue;
            }
            if self.bookmark_n >= 16 {
                skipped += 1;
                continue;
            }
            self.bookmark_paths.push(p);
            self.bookmark_n = self.bookmark_paths.len();
            added += 1;
        }
        self.save_bookmarks();
        if added > 0 {
            self.set_status(format!("added {added} shortcut(s) to Places"));
        } else if skipped > 0 {
            self.set_status(format!(
                "no shortcut added — {skipped} skipped (only folders allowed)"
            ));
        } else {
            self.set_status("nothing to add".into());
        }
        self.dirty = true;
    }

    #[allow(clippy::vec_init_then_push)]
    pub fn open_ribbon(&mut self, which: usize) {
        let labels = layout::RIBBON;        if which >= labels.len() {
            return;
        }
        // x position of ribbon label
        let mut bx = self.cfg.padding;
        let mut mx = bx;
        for (i, lab) in labels.iter().enumerate() {
            let w = self.text.as_ref().map(|t| t.width(lab) as i32).unwrap_or(40) + self.cfg.padding * 2;
            if i == which {
                mx = bx;
                break;
            }
            bx += w + 4;
        }
        let my = layout::menu_bar_h(self);
        let mut items = Vec::new();
        match which {
            0 => {
                // File
                items.push(Menu::item(MenuId::NewTab, "New Tab\tCtrl+T", true));
                items.push(Menu::item(MenuId::NewWindow, "New Window\tCtrl+N", true));
                items.push(Menu::item(MenuId::NewFolder, "New Folder…", true));
                items.push(Menu::item(MenuId::NewFile, "New File…", true));
                items.push(Menu::sep());
                items.push(Menu::item(MenuId::OpenTerminal, "Open Terminal", true));
                items.push(Menu::sep());
                items.push(Menu::item(MenuId::CloseTab, "Close Tab\tCtrl+W", true));
                items.push(Menu::item(MenuId::Quit, "Quit\tCtrl+Q", true));
            }
            1 => {
                items.push(Menu::item(MenuId::Undo, "Undo copy/move\tCtrl+Z", !self.undo_stack.is_empty()));
                items.push(Menu::sep());
                items.push(Menu::item(MenuId::Cut, "Cut\tCtrl+X", true));
                items.push(Menu::item(MenuId::Copy, "Copy\tCtrl+C", true));
                items.push(Menu::item(MenuId::Paste, "Paste\tCtrl+V", true));
                items.push(Menu::item(MenuId::Duplicate, "Duplicate Here\tCtrl+D", true));
                items.push(Menu::sep());
                items.push(Menu::item(MenuId::SelectAll, "Select All\tCtrl+A", true));
                items.push(Menu::item(MenuId::Rename, "Rename…\tF2", true));
                items.push(Menu::item(MenuId::Delete, "Delete", true));
                items.push(Menu::sep());
                items.push(Menu::item(MenuId::CopyFullPath, "Copy Full Path", true));
                items.push(Menu::item(MenuId::CopyBaseName, "Copy Base Name", true));
                items.push(Menu::item(MenuId::Properties, "Properties\tCtrl+Enter", true));
                // Custom Actions
                items.push(Menu::sep());
                items.push(Menu::item(MenuId::MenuHeader, "Custom Actions", false));
                items.push(Menu::item(MenuId::ConfigureActions, "Configure Custom Actions…", true));
                items.push(Menu::item(
                    MenuId::ImportThunar,
                    "Import Thunar Custom Actions…",
                    true,
                ));
            }
            2 => {
                items.push(Menu::item(MenuId::ViewList, "List View", true));
                items.push(Menu::item(MenuId::ViewGrid, "Grid View", true));
                items.push(Menu::item(MenuId::ViewCompact, "Compact View", true));
                items.push(Menu::sep());
                items.push(self.sort_submenu_item());
                items.push(Menu::sep());
                items.push(Menu::item(
                    MenuId::ToggleHidden,
                    if self.cfg.show_hidden {
                        "● Show Hidden Files\tCtrl+H"
                    } else {
                        "Show Hidden Files\tCtrl+H"
                    },
                    true,
                ));
                items.push(Menu::item(
                    MenuId::ToggleSplit,
                    if self.tab().split {
                        "● Split View\tF5"
                    } else {
                        "Split View\tF5"
                    },
                    true,
                ));
                items.push(Menu::item(
                    MenuId::ToggleExt,
                    if self.cfg.show_ext {
                        "● Show File Extensions"
                    } else {
                        "Show File Extensions"
                    },
                    true,
                ));
                items.push(Menu::item(MenuId::ConfigureToolbar, "Configure Toolbar…", true));
                // Hidden Places: untick a hidden default shortcut to restore it
                if !self.hidden_places.is_empty() {
                    items.push(Menu::sep());
                    items.push(Menu::item(MenuId::HiddenPlace(-1), "Hidden Places", false));
                    for (i, key) in self.hidden_places.iter().enumerate() {
                        let id = layout::place_id(key);
                        let label = if id >= 0 {
                            place_label(id as u8)
                        } else {
                            key.clone()
                        };
                        items.push(Menu::item(
                            MenuId::HiddenPlace(i as i32),
                            format!("● {label}"),
                            true,
                        ));
                    }
                }
                items.push(Menu::sep());
                items.push(Menu::item(MenuId::ZoomIn, "Zoom In\tCtrl++", true));
                items.push(Menu::item(MenuId::ZoomOut, "Zoom Out\tCtrl+-", true));
                items.push(Menu::sep());
                items.push(Menu::item(MenuId::ToggleSidebar, "Toggle Sidebar\tB", true));
                let prev_lab = if self.preview_visible {
                    "Hide Preview\tF8"
                } else {
                    "Show Preview\tF8"
                };
                items.push(Menu::item(MenuId::TogglePreview, prev_lab, true));
                items.push(Menu::item(MenuId::Refresh, "Reload\tCtrl+R", true));
            }
            3 => {
                // Theme
                let current = self.cfg.theme.clone();
                items.push(Menu::item(
                    MenuId::ThemeDark,
                    if current == "dark" { "● Dark Theme" } else { "Dark Theme" },
                    true,
                ));
                items.push(Menu::item(
                    MenuId::ThemeLight,
                    if current == "light" { "● Light Theme" } else { "Light Theme" },
                    true,
                ));
                self.theme_list = self.external_themes();
                if !self.theme_list.is_empty() {
                    items.push(Menu::sep());
                    for (i, spec) in self.theme_list.iter().enumerate() {
                        let stem = std::path::Path::new(spec)
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| spec.clone());
                        let mark = if current == *spec { "● " } else { "" };
                        items.push(Menu::item(MenuId::ThemeExt(i as i32), format!("{mark}{stem}"), true));
                    }
                }
                items.push(Menu::sep());
                items.push(Menu::item(MenuId::ToggleSidebar, "Toggle Sidebar\tB", true));
                let prev_lab = if self.preview_visible {
                    "Hide Preview\tF8"
                } else {
                    "Show Preview\tF8"
                };
                items.push(Menu::item(MenuId::TogglePreview, prev_lab, true));
            }
            4 => {
                items.push(Menu::item(MenuId::GoBack, "Back", true));
                items.push(Menu::item(MenuId::GoForward, "Forward", true));
                items.push(Menu::item(MenuId::GoUp, "Open Parent", true));
                items.push(Menu::item(MenuId::GoHome, "Home", true));
                items.push(Menu::item(MenuId::GoRoot, "File System", true));
            }
            5 => {
                items.push(Menu::item(MenuId::AddShortcutCwd, "Add Current Folder", true));
                for (i, p) in self.bookmark_paths.iter().enumerate() {
                    let lab = p
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| p.display().to_string());
                    items.push(Menu::item(MenuId::Open, format!("→ {lab}"), true));
                    let _ = i;
                }
            }
            _ => {
                items.push(Menu::item(MenuId::HelpAbout, "About wfm", true));
            }
        }
        let mut menu = Menu {
            x: mx,
            y: my,
            items,
            hover: -1,
            is_context: false,
            sub_open: None,
        };
        // tag shortcut targets: store paths in labels already; Open on shortcuts ribbon uses index via status — handle in activate
        self.ribbon_which = which as i32;
        self.clamp_menu(&mut menu);
        self.menu = Some(menu);
        self.dirty = true;
    }

    /// Collapse sidebar/preview on narrow windows so content stays usable.
    pub fn apply_responsive_layout(&mut self) {
        let w = self.width as i32;
        if w <= 0 {
            return;
        }
        // split panes need ~320px each — turn split off when too narrow
        if self.tab().split && layout::content_w(self) < 640 {
            self.tab_mut().split = false;
        }
        // Prefer user preference for preview; force-hide only when very narrow.
        if w < 520 {
            if self.sidebar_visible {
                self.sidebar_visible = false;
            }
            if self.preview_visible {
                self.preview_visible = false;
            }
        } else if w < 720 {
            // Keep preference for preview, but drop sidebar if it would crush the list.
            let side = if self.sidebar_visible {
                self.cfg.sidebar_width.min(w / 3).max(80)
            } else {
                0
            };
            let need = 200 + if self.cfg.show_preview { 120 } else { 0 };
            if side + need > w {
                self.sidebar_visible = false;
            }
            self.preview_visible = self.cfg.show_preview && (w - if self.sidebar_visible { self.cfg.sidebar_width.min(w / 3) } else { 0 }) >= 320;
        } else {
            // Restore from preferences when room allows.
            self.sidebar_visible = self.cfg.sidebar;
            self.preview_visible = self.cfg.show_preview;
        }
        if self.sidebar_visible {
            self.cfg.sidebar_width = layout::clamp_sidebar_width(self, self.cfg.sidebar_width);
        }
        if self.preview_visible {
            self.cfg.preview_width = layout::clamp_preview_width(self, self.cfg.preview_width);
        }
    }

    fn clamp_menu(&self, menu: &mut Menu) {
        let mw = layout::menu_width(self, menu);
        let mh = layout::menu_height(self, menu);
        let max_x = (self.width as i32 - mw).max(0);
        let max_y = (self.height as i32 - mh).max(0);
        if menu.x > max_x {
            menu.x = max_x;
        }
        if menu.y > max_y {
            menu.y = max_y;
        }
        if menu.x < 0 {
            menu.x = 0;
        }
        if menu.y < 0 {
            menu.y = 0;
        }
    }

    pub fn activate_menu_index(&mut self, idx: usize) {
        let Some(menu) = self.menu.take() else { return };
        if idx >= menu.items.len() {
            self.menu = Some(menu);
            return;
        }
        let item = menu.items[idx].clone();
        let is_context = menu.is_context;
        if !item.enabled || item.id == MenuId::Separator {
            self.menu = Some(menu);
            return;
        }
        // submenu opener (e.g. Sort By): toggle the sub open/closed
        if item.sub.is_some() {
            let mut menu = menu;
            menu.sub_open = if menu.sub_open == Some(idx) { None } else { Some(idx) };
            self.menu = Some(menu);
            let open = self.menu.as_ref().and_then(|m| m.sub_open);
            self.set_sub_open(open);
            self.dirty = true;
            return;
        }
        // Shortcuts ribbon: Open items after first are bookmark paths
        if !is_context && self.ribbon_which == 5 && item.id == MenuId::Open && idx >= 1 {
            let bi = idx - 1;
            if bi < self.bookmark_paths.len() {
                let p = self.bookmark_paths[bi].clone();
                self.cd(p);
            }
            self.dirty = true;
            return;
        }
        // remember where the menu was so the Open With submenu lands nearby
        self.menu_x = menu.x;
        self.menu_y = menu.y;
        self.run_menu_id(item.id);
    }

    /// Resolve a pointer click over the open menu: the submenu floats on top,
    /// so it is tested first. Returns `Some((is_sub, parent_idx, sub_idx))`
    /// when the click lands on a menu item.
    fn menu_click_at(&self, x: i32, y: i32) -> Option<(bool, usize, usize)> {
        let menu = self.menu.as_ref()?;
        if let Some(oi) = menu.sub_open {
            if let Some(sub) = menu.items.get(oi).and_then(|it| it.sub.as_deref()) {
                let si = layout::menu_item_at(self, sub, x, y);
                if si >= 0 {
                    return Some((true, oi, si as usize));
                }
            }
        }
        let hi = layout::menu_item_at(self, menu, x, y);
        if hi >= 0 {
            return Some((false, hi as usize, 0));
        }
        None
    }

    /// Dispatch a click on an item inside the currently open submenu.
    fn activate_sub_index(&mut self, oi: usize, si: usize) {
        let Some(menu) = self.menu.as_ref() else { return };
        let Some(sub) = menu.items.get(oi).and_then(|it| it.sub.as_deref()) else { return };
        let Some(item) = sub.items.get(si).cloned() else { return };
        let mx = sub.x;
        let my = sub.y;
        if !item.enabled || item.id == MenuId::Separator {
            return;
        }
        self.menu = None;
        self.menu_x = mx;
        self.menu_y = my;
        self.run_menu_id(item.id);
        self.dirty = true;
    }

    /// Open/close the submenu of the item at `idx` in the current menu,
    /// positioning it beside the item row (right, or left near the edge).
    fn set_sub_open(&mut self, idx: Option<usize>) {
        if let Some(i) = idx {
            // geometry computed from an immutable snapshot first
            let (sx, sy) = {
                let Some(menu) = self.menu.as_ref() else { return };
                let Some(sub) = menu.items.get(i).and_then(|it| it.sub.as_deref()) else { return };
                let mw = layout::menu_width(self, menu);
                let sw = layout::menu_width(self, sub);
                let iy = layout::menu_item_y(self, menu, i);
                let mut sx = menu.x + mw;
                if sx + sw > self.width as i32 {
                    sx = (menu.x - sw).max(0);
                }
                let mut sy = menu.y + iy;
                let sh = layout::menu_height(self, sub);
                if sy + sh > self.height as i32 {
                    sy = (self.height as i32 - sh).max(0);
                }
                (sx, sy)
            };
            if let Some(menu) = self.menu.as_mut() {
                menu.sub_open = Some(i);
                if let Some(sub) = menu.items.get_mut(i).and_then(|it| it.sub.as_deref_mut()) {
                    sub.x = sx;
                    sub.y = sy;
                    sub.hover = -1;
                }
            }
        } else if let Some(menu) = self.menu.as_mut() {
            menu.sub_open = None;
        }
    }

    /// Hover tracking for the open menu and its submenu. The submenu (if any)
    /// has priority; moving to a different parent item switches/closes it.
    fn menu_hover_at(&mut self, x: i32, y: i32) {
        let Some(menu) = self.menu.as_ref() else { return };
        let oi = menu.sub_open;
        // pointer inside the open submenu?
        if let Some(s) = oi.and_then(|i| menu.items.get(i).and_then(|it| it.sub.clone())) {
            let sh = layout::menu_item_at(self, &s, x, y);
            if sh >= 0 {
                let changed = self.menu_hover != oi.unwrap() as i32 || s.hover != sh;
                self.menu_hover = oi.unwrap() as i32;
                if let Some(m) = self.menu.as_mut() {
                    m.hover = oi.unwrap() as i32;
                    if let Some(ss) = m.items.get_mut(oi.unwrap()).and_then(|it| it.sub.as_deref_mut()) {
                        ss.hover = sh;
                    }
                }
                if changed {
                    self.dirty = true;
                }
                return;
            }
        }
        let h = layout::menu_item_at(self, menu, x, y);
        if h >= 0 {
            let target = if menu.items[h as usize].sub.is_some() {
                Some(h as usize)
            } else {
                None
            };
            if target != oi {
                self.set_sub_open(target);
            }
            let changed = self.menu_hover != h;
            self.menu_hover = h;
            if let Some(m) = self.menu.as_mut() {
                m.hover = h;
            }
            if changed || target != oi {
                self.dirty = true;
            }
        } else {
            if oi.is_some() {
                self.set_sub_open(None);
            }
            let changed = self.menu_hover != -1;
            self.menu_hover = -1;
            if let Some(m) = self.menu.as_mut() {
                m.hover = -1;
            }
            if changed || oi.is_some() {
                self.dirty = true;
            }
        }
    }

    /// Scan the user theme dirs for `*.theme` files (config + data home).
    pub fn external_themes(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut dirs = vec![crate::config::themes_dir()];
        if let Some(h) = std::env::var_os("XDG_DATA_HOME") {
            if !h.is_empty() {
                dirs.push(std::path::PathBuf::from(h).join("wfm/themes"));
            }
        } else if let Some(h) = std::env::var_os("HOME") {
            dirs.push(std::path::PathBuf::from(h).join(".local/share/wfm/themes"));
        }
        for d in dirs {
            if let Ok(rd) = std::fs::read_dir(&d) {
                for ent in rd.flatten() {
                    let p = ent.path();
                    if p.extension().map(|e| e == "theme").unwrap_or(false) {
                        let s = p.display().to_string();
                        if !out.contains(&s) {
                            out.push(s);
                        }
                    }
                }
            }
        }
        out
    }

    /// Resolve a theme spec, apply it live and persist `theme =` to the config.
    fn apply_theme_spec(&mut self, spec: &str) {
        let t = crate::theme::resolve(spec);
        t.apply_to_cfg(&mut self.cfg);
        self.cfg.theme = spec.to_string();
        if let Some(p) = self.cfg_path.clone() {
            crate::config::set_theme(&p, spec);
        }
        self.dirty = true;
        self.set_status(format!("theme: {spec}"));
    }

    pub fn run_menu_id(&mut self, id: MenuId) {
        match id {
            MenuId::Open => self.open_selected(),
            MenuId::OpenWith => self.open_open_with(),
            MenuId::OpenWithApp(i) => {
                let Some((_, exec, _)) = self.open_with_list.get(i as usize).cloned() else {
                    return;
                };
                let Some(path) = self.selected_paths().first().cloned() else {
                    return;
                };
                if fs::open_with_cmd(&path, &exec).is_err() {
                    self.set_err(format!("failed to launch {exec}"));
                } else {
                    self.set_status(format!("open with: {exec}"));
                    if !self.tab().is_virtual() {
                        fs::recent_add(&path);
                    }
                }
            }
            MenuId::OpenWithCustom => {
                let mime = self
                    .tab()
                    .selected()
                    .map(|e| fs::mime_for_path(&fs::full_path(&self.tab().cwd, &e.name)))
                    .unwrap_or_default();
                let cur = fs::default_app_for(&mime).unwrap_or_else(|| self.cfg.opener.clone());
                self.begin_input(InputMode::OpenWith, "open with:", &cur);
            }
            MenuId::SetDefaultApp => {
                let mime = self
                    .tab()
                    .selected()
                    .map(|e| fs::mime_for_path(&fs::full_path(&self.tab().cwd, &e.name)))
                    .unwrap_or_default();
                let cur = fs::default_app_for(&mime).unwrap_or_else(|| self.cfg.opener.clone());
                self.begin_input(InputMode::SetDefault, "default app:", &cur);
            }
            MenuId::OpenTerminal => self.open_terminal_here(),
            MenuId::Cut => self.clip_cut(),
            MenuId::Copy => self.clip_copy(),
            MenuId::Paste => self.clip_paste(),
            MenuId::Undo => self.undo_last(),
            MenuId::UrlUndo => self.url_undo_pop(),
            MenuId::UrlCut => self.url_cut(),
            MenuId::UrlCopy => self.url_copy(),
            MenuId::UrlPaste => self.url_paste(),
            MenuId::UrlClear => self.url_clear(),
            MenuId::UrlSelectAll => self.url_select_all(),
            MenuId::UrlHistory(i) => {
                let p = self.url_history.get(i as usize).cloned();
                self.url_go_from(p.as_ref());
            }
            MenuId::UrlSuggest(i) => {
                let p = self.url_sug_list.get(i as usize).cloned();
                self.url_go_from(p.as_ref());
            }
            MenuId::SortName => self.set_sort(crate::tab::SortKey::Name),
            MenuId::SortSize => self.set_sort(crate::tab::SortKey::Size),
            MenuId::SortType => self.set_sort(crate::tab::SortKey::Type),
            MenuId::SortMtime => self.set_sort(crate::tab::SortKey::Mtime),
            MenuId::SortDesc => self.toggle_sort_desc(),
            MenuId::Duplicate => self.duplicate_selected(),
            MenuId::Rename => {
                let name = self.tab().selected().map(|e| e.name.clone()).unwrap_or_default();
                if !name.is_empty() {
                    self.begin_input(InputMode::Rename, "", &name);
                }
            }
            MenuId::Delete => self.delete_selected(),
            MenuId::Properties => {
                if let Some(p) = self.ctx_path.clone() {
                    self.open_props_for(&p);
                } else {
                    self.open_props_selected();
                }
            }
            MenuId::CopyFullPath | MenuId::CopyBaseName | MenuId::CopyParent | MenuId::CopyStem => {
                self.copy_path_text(id)
            }
            MenuId::GoToLinkTarget => self.go_link_target(),
            MenuId::AddShortcut => {
                // selected items, or the current folder when nothing is selected
                let paths = self.selected_paths();
                if paths.is_empty() {
                    self.toggle_shortcut_cwd();
                } else {
                    self.add_shortcuts(paths);
                }
            }
            MenuId::AddShortcutCwd => self.toggle_shortcut_cwd(),
            MenuId::RemoveShortcut => self.toggle_shortcut_cwd(),
            MenuId::HidePlace => {
                if let Some(key) = self.ctx_place_key.clone() {
                    self.hide_place(&key);
                }
            }
            MenuId::HiddenPlace(i) => {
                // View → Hidden Places: visible hidden keys, restore on click
                let key = self.hidden_places.get(i as usize).cloned();
                if let Some(key) = key {
                    self.show_place(&key);
                }
            }
            MenuId::Mount => {
                if let Some(dev) = self.ctx_path.clone() {
                    let name = dev
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "vol".into());
                    let mp = PathBuf::from(format!("/media/{name}"));
                    self.set_status(format!("mounting {}…", dev.display()));
                    if fs::mount_with_polkit(&dev, &mp) {
                        self.sidebar_refresh();
                        self.cd(mp);
                        self.set_status(format!("mounted {}", dev.display()));
                    } else {
                        self.set_err(format!("failed to mount {} (polkit?)", dev.display()));
                    }
                }
            }
            MenuId::Unmount => {
                if let Some(p) = self.ctx_path.clone() {
                    self.set_status(format!("unmounting {}…", p.display()));
                    if fs::unmount(&p) {
                        self.sidebar_refresh();
                        self.set_status(format!("unmounted {}", p.display()));
                    } else {
                        self.set_err(format!("failed to unmount {}", p.display()));
                    }
                }
            }
            MenuId::OpenNewTab => {
                if let Some(p) = self.ctx_path.clone() {
                    self.new_tab_at(p);
                }
            }
            MenuId::OpenSplit => {
                if let Some(p) = self.ctx_path.clone() {
                    self.open_split_at(p);
                }
            }
            MenuId::NewFolder => self.begin_input(InputMode::NewDir, "", "New Folder"),
            MenuId::NewFile => self.begin_input(InputMode::NewFile, "", ""),
            MenuId::SelectAll => self.select_all_visible(),
            MenuId::NewTab => self.new_tab(),
            MenuId::NewWindow => {
                let dir = self.ctx_path.clone();
                self.spawn_new_window(dir);
            }
            MenuId::CloseTab => self.close_tab(),
            MenuId::Quit => self.request_close(),
            MenuId::ToggleSidebar => {
                self.sidebar_visible = !self.sidebar_visible;
                self.cfg.sidebar = self.sidebar_visible;
                self.dirty = true;
                self.save_state();
            }
            MenuId::TogglePreview => {
                self.preview_visible = !self.preview_visible;
                self.cfg.show_preview = self.preview_visible;
                self.dirty = true;
                self.save_state();
            }
            MenuId::ToggleSplit => self.toggle_split(),
            MenuId::ToggleHidden => self.toggle_hidden(),
            MenuId::ToggleExt => {
                self.cfg.show_ext = !self.cfg.show_ext;
                if let Some(p) = &self.cfg_path {
                    let val = if self.cfg.show_ext { "true" } else { "false" };
                    crate::config::set_keys(p, &[("show_ext", val.to_string())]);
                }
                self.dirty = true;
            }
            MenuId::ConfigureToolbar => self.open_toolbar_dialog(),
            MenuId::ViewList => {
                self.tab_mut().view = ViewMode::List;
                self.dirty = true;
                self.save_state();
            }
            MenuId::ViewGrid => {
                self.tab_mut().view = ViewMode::Grid;
                self.dirty = true;
                self.save_state();
            }
            MenuId::ViewCompact => {
                self.tab_mut().view = ViewMode::Compact;
                self.dirty = true;
                self.save_state();
            }
            MenuId::ZoomIn => self.zoom_icons(1),
            MenuId::ZoomOut => self.zoom_icons(-1),
            MenuId::GoHome => {
                let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"));
                self.cd(home);
            }
            MenuId::GoRoot => self.cd(PathBuf::from("/")),
            MenuId::GoUp => self.cd_parent(),
            MenuId::GoBack => {
                if self.nav_back() {
                    self.reload();
                    self.reload_status();
                }
            }
            MenuId::GoForward => {
                if self.nav_fwd() {
                    self.reload();
                    self.reload_status();
                }
            }
            MenuId::Refresh => self.reload(),
            MenuId::HelpAbout => self.set_status(format!("wfm {} — pure Wayland file manager", env!("CARGO_PKG_VERSION"))),
            MenuId::ThemeDark => self.apply_theme_spec("dark"),
            MenuId::ThemeLight => self.apply_theme_spec("light"),
            MenuId::ThemeExt(i) => {
                if let Some(spec) = self.theme_list.get(i as usize) {
                    let spec = spec.clone();
                    self.apply_theme_spec(&spec);
                }
            }
            MenuId::DndCopy => self.dnd_execute(0),
            MenuId::DndMove => self.dnd_execute(1),
            MenuId::DndLink => self.dnd_execute(2),
            MenuId::DndLinkRel => self.dnd_execute(3),
            MenuId::DndHardlink => self.dnd_execute(4),
            MenuId::DndMoveNew => self.begin_dnd_new(0),
            MenuId::DndCopyNew => self.begin_dnd_new(1),
            MenuId::DndCancel => self.dnd_clear(),
            MenuId::CustomAction(i) => {
                let Some((name, cmd)) = self.custom_actions.get(i as usize).cloned() else {
                    return;
                };
                let paths = self.selected_paths();
                if paths.is_empty() {
                    return;
                }
                let cwd = paths[0]
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| self.tab().cwd.clone());
                let expanded = crate::actions::expand_command(&cmd, &paths);
                let r = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&expanded)
                    .current_dir(&cwd)
                    .spawn();
                if r.is_err() {
                    self.set_err(format!("failed to run custom action: {name}"));
                } else {
                    self.set_status(format!("custom action: {name}"));
                }
            }
            MenuId::ConfigureActions => {
                let path = crate::actions::ensure_default_actions();
                if fs::open_with_cmd(&path, &self.cfg.opener).is_err() {
                    self.set_err(format!("failed to open {}", path.display()));
                } else {
                    self.set_status(format!("custom actions: {}", path.display()));
                }
            }
            MenuId::ImportThunar => {
                match crate::actions::import_from_thunar() {
                    Ok(actions) => {
                        let existing = crate::actions::load_custom_actions();
                        if existing.is_empty() {
                            // nothing to lose → import straight away
                            match crate::actions::write_actions(&actions) {
                                Ok(()) => {
                                    self.set_status(format!(
                                        "imported {} custom action(s) from Thunar",
                                        actions.len()
                                    ));
                                }
                                Err(e) => self.set_err(format!("import failed: {e}")),
                            }
                        } else {
                            // warn: importing replaces current actions
                            self.import_pending = actions;
                            self.confirm_import = true;
                            self.dirty = true;
                        }
                    }
                    Err(msg) => self.set_err(msg),
                }
            }
            MenuId::MenuHeader | MenuId::Separator => {}
            MenuId::SortBy => {} // submenu opener — handled by the menu, never dispatched
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_app() -> App {
        App::new(Config::default())
    }

    /// Sidebar shortcuts accept only directories — or symlinks resolving to
    /// a directory. Files, file-symlinks and broken symlinks are rejected,
    /// both via the toggle and via DnD `add_shortcuts`.
    #[test]
    fn shortcuts_only_accept_directories() {
        // keep the real bookmarks file untouched
        let bpath = App::bookmarks_path();
        let saved = std::fs::read_to_string(&bpath).unwrap_or_default();

        let base = std::env::temp_dir().join(format!("wfm-shortcut-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("adir")).unwrap();
        std::fs::write(base.join("afile"), "x").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(base.join("adir"), base.join("link_dir")).unwrap();
            std::os::unix::fs::symlink(base.join("afile"), base.join("link_file")).unwrap();
            std::os::unix::fs::symlink(base.join("gone"), base.join("link_broken")).unwrap();
        }

        let mut a = scratch_app();
        a.toggle_shortcut_path(base.join("afile"));
        assert_eq!(a.bookmark_n, 0, "a file must not become a shortcut");

        #[cfg(unix)]
        {
            a.toggle_shortcut_path(base.join("link_file"));
            assert_eq!(a.bookmark_n, 0, "symlink to a file must be rejected");
            a.toggle_shortcut_path(base.join("link_broken"));
            assert_eq!(a.bookmark_n, 0, "broken symlink must be rejected");
            a.toggle_shortcut_path(base.join("link_dir"));
            assert_eq!(a.bookmark_n, 1, "symlink to a dir must be accepted");
        }

        a.toggle_shortcut_path(base.join("adir"));
        assert_eq!(a.bookmark_n, 2, "a directory must be accepted");
        a.toggle_shortcut_path(base.join("adir"));
        assert_eq!(a.bookmark_n, 1, "toggling again removes the shortcut");

        // DnD add_shortcuts skips non-directories silently
        let mut b = scratch_app();
        b.add_shortcuts(vec![base.join("afile"), base.join("adir")]);
        assert_eq!(b.bookmark_n, 1);
        assert!(b.bookmark_paths.contains(&base.join("adir")));

        let _ = std::fs::remove_dir_all(&base);
        let _ = std::fs::write(&bpath, saved);
    }

    /// Esc in the idle state always clears every selection (multi-select,
    /// rubberband, etc.) — nothing else happens.
    #[test]
    fn esc_clears_all_selections() {
        let mut a = scratch_app();
        let mut t = crate::tab::Tab::new(1, std::path::PathBuf::from("/tmp"));
        t.entries = vec![
            crate::entry::Entry {
                name: "a".into(),
                selected: true,
                ..Default::default()
            },
            crate::entry::Entry {
                name: "b".into(),
                selected: true,
                ..Default::default()
            },
            crate::entry::Entry {
                name: "c".into(),
                selected: false,
                ..Default::default()
            },
        ];
        t.rebuild_visible();
        a.tabs.push(t);
        a.cur_tab = 0;
        a.count_selected();
        assert_eq!(a.n_sel, 2);

        a.on_key(crate::wl::Keysym::Escape, None, false, false);
        assert!(a.tab().entries.iter().all(|e| !e.selected));
        assert!(a.tab().sel_none);
        assert_eq!(a.n_sel, 0);
    }

    /// Restoring a saved split must open the right pane *inside* the tab —
    /// split is per-tab, so no second tab is created.
    #[test]
    fn apply_state_restores_split_as_embedded_pane() {
        let mut a = scratch_app();
        let t = crate::tab::Tab::new(0, std::path::PathBuf::from("/tmp"));
        a.tabs.push(t);
        a.cur_tab = 0;
        assert_eq!(a.tabs.len(), 1);

        let mut s = crate::config::State::default();
        s.split = true;
        a.apply_state(&s);

        assert!(a.tab().split, "split flag restored on the tab");
        assert!(a.tab().pane.is_some(), "an embedded right pane is created");
        assert_eq!(a.tabs.len(), 1, "no second tab is created");
        assert!(layout::split_active(&a));
    }

    /// Split is per-tab: toggling it on one tab embeds a right pane without
    /// adding a tab, and focusing the right pane swaps the panes (so keyboard
    /// input keeps acting on the focused pane) while staying a single tab.
    #[test]
    fn toggle_split_embeds_pane_and_focus_swaps_it() {
        let mut a = scratch_app();
        let t = crate::tab::Tab::new(0, std::path::PathBuf::from("/tmp"));
        a.tabs.push(t);
        a.cur_tab = 0;
        a.worker = None;

        a.toggle_split();
        assert!(a.tab().split, "split toggled on");
        assert!(a.tab().pane.is_some(), "embedded pane created");
        assert_eq!(a.tabs.len(), 1, "no second tab");
        assert!(layout::split_active(&a));
        let pane_id = a.tab().pane.as_ref().unwrap().id;
        assert_ne!(pane_id, a.tab().id, "pane has its own tab id");

        let left_cwd = a.tab().cwd.clone();
        let right_cwd = a.tab().pane.as_ref().unwrap().cwd.clone();
        a.focus_right_pane();
        assert_eq!(a.tab().cwd, right_cwd, "focused tab is now the old pane");
        assert!(a.tab().split, "split preserved after focus swap");
        let new_right = a.tab().pane.as_ref().unwrap().cwd.clone();
        assert_eq!(new_right, left_cwd, "old tab moved to the right pane");
        assert_eq!(a.tabs.len(), 1, "still one tab");

        a.toggle_split();
        assert!(!a.tab().split, "second toggle turns split off");
        assert!(!layout::split_active(&a));
    }

    fn typeahead_app() -> App {
        let mut a = scratch_app();
        let mut t = crate::tab::Tab::new(1, std::path::PathBuf::from("/tmp"));
        t.entries = vec![
            crate::entry::Entry { name: "apple".into(), ..Default::default() },
            crate::entry::Entry { name: "banana".into(), ..Default::default() },
            crate::entry::Entry { name: "avocado".into(), ..Default::default() },
        ];
        t.rebuild_visible();
        a.tabs.push(t);
        a.cur_tab = 0;
        a.count_selected();
        a
    }

    /// Typing while idle selects the first entry whose name starts with the
    /// typed text (case-insensitive); once the box is open every printable
    /// char extends it; Backspace shortens, Esc dismisses.
    #[test]
    fn type_to_select_matches_prefix() {
        let mut a = typeahead_app();

        // 'a' is unbound, so it starts the typeahead
        a.on_key(crate::wl::Keysym::a, Some("a".to_string()), false, false);
        assert_eq!(a.typeahead, "a");
        assert_eq!(a.tab().sel, 0, "apple is the first 'a' match");

        // 'v' is a shortcut key, but while the box is open it extends it
        a.on_key(crate::wl::Keysym::v, Some("v".to_string()), false, false);
        assert_eq!(a.typeahead, "av");
        assert_eq!(a.tab().sel, 2, "avocado matches 'av'");

        // Backspace shrinks back to "a"
        a.on_key(crate::wl::Keysym::BackSpace, None, false, false);
        assert_eq!(a.typeahead, "a");
        assert_eq!(a.tab().sel, 0);

        // no match clears the selection but keeps the box open
        a.on_key(crate::wl::Keysym::c, Some("c".to_string()), false, false);
        assert_eq!(a.typeahead, "ac");
        assert!(a.tab().sel_none, "no entry starts with 'ac'");

        // Esc dismisses the box
        a.on_key(crate::wl::Keysym::Escape, None, false, false);
        assert!(a.typeahead.is_empty());
    }

    /// Keys taken by single-letter commands must not start the typeahead.
    #[test]
    fn type_to_select_respects_shortcut_keys() {
        let mut a = typeahead_app();

        // 'n' is New Folder — it must fire that, not a typeahead
        a.on_key(crate::wl::Keysym::n, Some("n".to_string()), false, false);
        assert!(a.typeahead.is_empty());
        assert_eq!(a.input_mode, crate::app::InputMode::NewDir);
    }

    /// The Configure Toolbar dialog edits working copies: unchecking hides an
    /// item, moving changes the order, and applying writes back to cfg.
    #[test]
    fn toolbar_dialog_toggle_move_apply() {
        let mut a = scratch_app();
        a.custom_actions = vec![("Open in Terminal".to_string(), "sh -c 'xterm'".to_string())];
        a.open_toolbar_dialog();
        let dlg = a.toolbar_dlg.as_ref().expect("dialog opened");
        assert_eq!(dlg.items.len(), crate::toolbar::default_toolbar().len() + 1);
        // the new custom action starts hidden (unchecked)
        assert!(dlg.is_hidden("ca:Open in Terminal"));
        // uncheck (hide) the Search button
        let search_idx = dlg
            .items
            .iter()
            .position(|it| *it == crate::toolbar::ToolbarItem::Search)
            .unwrap();
        a.toolbar_dlg.as_mut().unwrap().toggle(search_idx);
        assert!(a.toolbar_dlg.as_ref().unwrap().is_hidden("search"));
        // move the hidden Search after Up
        a.toolbar_dlg.as_mut().unwrap().toggle(search_idx); // re-check first
        let up_idx = a
            .toolbar_dlg
            .as_ref()
            .unwrap()
            .items
            .iter()
            .position(|it| *it == crate::toolbar::ToolbarItem::Up)
            .unwrap();
        a.toolbar_dlg.as_mut().unwrap().sel = search_idx;
        a.toolbar_dlg.as_mut().unwrap().move_sel(-1);
        let items = &a.toolbar_dlg.as_ref().unwrap().items;
        assert_eq!(items[search_idx - 1], crate::toolbar::ToolbarItem::Search);
        assert_eq!(items[up_idx], crate::toolbar::ToolbarItem::Up);
        // applying persists into cfg and resets the dialog
        a.apply_toolbar_dialog();
        assert!(a.toolbar_dlg.is_none());
        assert!(!a.cfg.toolbar_hidden.contains(&"search".to_string()));
        assert!(a.cfg.toolbar_hidden.contains(&"ca:Open in Terminal".to_string()));
        assert_eq!(a.cfg.toolbar[search_idx - 1], crate::toolbar::ToolbarItem::Search);
    }

    /// Esc closes the Configure Toolbar dialog without applying.
    #[test]
    fn toolbar_dialog_esc_cancels() {
        let mut a = scratch_app();
        a.open_toolbar_dialog();
        let before = a.cfg.toolbar.clone();
        a.on_key(crate::wl::Keysym::Escape, None, false, false);
        assert!(a.toolbar_dlg.is_none());
        assert_eq!(a.cfg.toolbar, before);
    }
}
