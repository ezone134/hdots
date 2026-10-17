pub const MAX_ENTRIES: usize = 8192;
pub const WFM_MAX_INPUT: usize = 256;
/// Max thumbnails generated per listing pass.
pub const MAX_THUMB: usize = 48;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EntryType {
    #[default]
    File,
    Dir,
    Link,
    Image,
    Archive,
}

#[derive(Clone, Default)]
pub struct Entry {
    pub name: String,
    pub kind: EntryType,
    pub size: u64,
    pub mtime: i64,
    pub is_dir: bool,
    pub is_link: bool,
    /// File with more than one hard link (st_nlink > 1).
    pub is_hardlink: bool,
    /// Regular file with at least one execute bit set (drives the exec icon).
    pub is_exec: bool,
    /// Original path for virtual listings (Recent/Trash); None for real dirs.
    pub real_path: Option<std::path::PathBuf>,
    pub thumb: Option<Vec<u8>>,
    pub thumb_w: i32,
    pub thumb_h: i32,
    pub thumb_pending: bool,
    pub selected: bool,
}
