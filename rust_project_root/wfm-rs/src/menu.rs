//! Context menu + top ribbon (Thunar-style), pure software popup.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuId {
    Open,
    OpenTerminal,
    /// "Open With…" — opens the app picker popup.
    OpenWith,
    /// Open with a specific app from the picker list (index into
    /// `App::open_with_list`).
    OpenWithApp(i32),
    /// Open with a user-typed command (inline input).
    OpenWithCustom,
    /// Remember the given command as the default opener for this file type.
    SetDefaultApp,
    Cut,
    Copy,
    Paste,
    Duplicate,
    Rename,
    Delete,
    Properties,
    CopyFullPath,
    CopyBaseName,
    CopyParent,
    CopyStem,
    GoToLinkTarget,
    AddShortcut,
    /// Ribbon "Add Current Folder" — always adds the cwd (context-menu
    /// AddShortcut instead adds the selected items, or the cwd when none).
    AddShortcutCwd,
    RemoveShortcut,
    /// Hide a default place shortcut from the sidebar (right-click on it).
    HidePlace,
    /// Restore a hidden place from View → Hidden Places (index into the
    /// visible hidden-places list).
    HiddenPlace(i32),
    NewFolder,
    NewFile,
    SelectAll,
    // sidebar volume/place actions
    Mount,
    Unmount,
    OpenNewTab,
    OpenSplit,
    // ribbon-only
    NewTab,
    NewWindow,
    CloseTab,
    Quit,
    ToggleSidebar,
    TogglePreview,
    ToggleSplit,
    ToggleHidden,
    /// Toggle the "Show File Extensions" preference.
    ToggleExt,
    /// Open the Configure Toolbar dialog.
    ConfigureToolbar,
    ViewList,
    ViewGrid,
    ViewCompact,
    ZoomIn,
    ZoomOut,
    // theme selection (ribbon Theme section)
    ThemeDark,
    ThemeLight,
    ThemeExt(i32),
    GoHome,
    GoRoot,
    GoUp,
    GoBack,
    GoForward,
    Refresh,
    HelpAbout,
    // cross-window drag & drop popup
    DndCopy,
    DndMove,
    DndLink,
    DndMoveNew,
    DndCopyNew,
    DndLinkRel,
    DndHardlink,
    DndCancel,
    /// Custom action (index into `App::custom_actions`).
    CustomAction(i32),
    /// Open wfm's own custom-actions file (creating a template) in the editor.
    ConfigureActions,
    /// Temporary: import Thunar's uca.xml into wfm's own format. To be
    /// removed in a future release once users have migrated.
    ImportThunar,
    /// Disabled section header inside a ribbon menu (never activated).
    MenuHeader,
    /// Undo the last copy/move (Ctrl+Z).
    Undo,
    /// URL-bar edit actions (right-click in the inline location field).
    UrlUndo,
    UrlCut,
    UrlCopy,
    UrlPaste,
    UrlClear,
    UrlSelectAll,
    /// URL-bar dropdown: navigate to a history entry (index into
    /// `App::url_history`).
    UrlHistory(i32),
    /// URL-bar dropdown: navigate to a filesystem completion suggestion
    /// (index into `App::url_sug_list`).
    UrlSuggest(i32),
    /// Sort By group.
    /// Opener item that holds the sort submenu (never dispatched itself).
    SortBy,
    SortName,
    SortSize,
    SortType,
    SortMtime,
    SortDesc,
    Separator,
}

#[derive(Clone)]
pub struct MenuItem {
    pub id: MenuId,
    pub label: String,
    pub enabled: bool,
    /// Nested submenu shown to the side when this item is hovered/activated.
    pub sub: Option<Box<Menu>>,
}

#[derive(Clone)]
pub struct Menu {
    pub x: i32,
    pub y: i32,
    pub items: Vec<MenuItem>,
    pub hover: i32,
    /// true = context, false = ribbon dropdown
    pub is_context: bool,
    /// Index into `items` of the item whose submenu is currently open.
    pub sub_open: Option<usize>,
}

impl Menu {
    pub fn sep() -> MenuItem {
        MenuItem {
            id: MenuId::Separator,
            label: String::new(),
            enabled: false,
            sub: None,
        }
    }
    pub fn item(id: MenuId, label: impl Into<String>, enabled: bool) -> MenuItem {
        MenuItem {
            id,
            label: label.into(),
            enabled,
            sub: None,
        }
    }
    /// An item that opens `sub` to its side (e.g. "Sort By ›").
    pub fn submenu(id: MenuId, label: impl Into<String>, sub: Menu) -> MenuItem {
        MenuItem {
            id,
            label: label.into(),
            enabled: true,
            sub: Some(Box::new(sub)),
        }
    }
}
