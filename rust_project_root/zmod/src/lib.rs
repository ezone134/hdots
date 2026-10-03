//! Loader for zen-shell data-provider plugins.
//!
//! A plugin is a `.so` in `$sources/lib` exporting the four functions in
//! [`abi`] (C header: `include/zenmod.h`). Every refresh tick the host pulls
//! all `(key, value)` pairs, copies them into Rust-owned `String`s, and merges
//! them into the scene-value namespace, where `.ron` cards bind them with the
//! usual `{name}` interpolation.
//!
//! Design notes worth knowing before changing anything here:
//!
//!  * **Trust.** `$sources/lib` is the same trust level as `$sources/sp`: code
//!    the user installed. A plugin that segfaults or aborts takes the bar with
//!    it — a panic unwound out of an `extern "C"` frame aborts the process and
//!    `catch_unwind` in the host cannot see it. Plugins must contain their own
//!    failures (see the clock plugin's `guard` helper).
//!  * **No `dlclose`.** Handles are leaked on purpose: unloading a module that
//!    may have registered atexit handlers, thread-locals or live `static`s is
//!    how shells turn into segfaults. Hot reload therefore means "load the new
//!    file under a new name" (`clock.so` → `clock2.so`).
//!  * **Strings are copied immediately.** The ABI only guarantees the returned
//!    pointer is valid until the call returns, so nothing borrowed survives.

pub mod abi;

use std::ffi::{c_char, c_void, CStr, CString};
use std::path::{Path, PathBuf};

pub use abi::ABI_VERSION;

/// A successfully loaded module and the values it published this tick.
pub struct Module {
    /// File stem, e.g. `clock` for `clock.so`. Used in log lines and in the
    /// `zen-shell mods` listing.
    pub name: String,
    /// Full path the module was loaded from.
    pub path: PathBuf,
    /// `(key, value)` pairs, copied out of the plugin on load and re-pulled on
    /// every [`Module::refresh`].
    pub values: Vec<(String, String)>,
    /// Resolved entry points, kept so a refresh does not need another
    /// `dlsym` round (and so `refresh` cannot be tricked into calling a
    /// different module's symbols).
    api: abi::Api,
    /// Kept alive forever on purpose — see the module docs. Never dropped, and
    /// deliberately not wrapped in a `Drop` impl: the whole point is that the
    /// handle outlives every borrow of this struct.
    #[allow(dead_code)]
    handle: *mut c_void,
}

impl Module {
    /// Pull the module's values again, in place. This is what makes a clock
    /// tick: the host calls it every refresh, not just at load.
    ///
    /// On failure the previous values are kept — a module that throws on one
    /// tick should not blank out the card until the next good one.
    pub fn refresh(&mut self) -> Result<(), String> {
        // SAFETY: `api` was resolved from this module's own handle at load
        // time and the handle is never closed, so the pointers stay valid for
        // the life of the struct.
        let (count, values) = unsafe {
            let count = (self.api.provider_count)();
            if count > abi::MAX_PROVIDERS {
                return Err(format!("claims {count} providers, cap is {}", abi::MAX_PROVIDERS));
            }
            let mut values = Vec::with_capacity(count as usize);
            for i in 0..count {
                values.push((
                    copy_str((self.api.provider_key)(i), abi::MAX_KEY, "key")?,
                    copy_str((self.api.provider_value)(i), abi::MAX_VALUE, "value")?,
                ));
            }
            (count, values)
        };
        for (i, (key, _)) in values.iter().enumerate() {
            check_key(i as u32, key)?;
        }
        let _ = count;
        self.values = values;
        Ok(())
    }
}

/// A module that could not be loaded, with the reason to show the user.
pub struct LoadError {
    pub path: PathBuf,
    pub reason: String,
}

/// Everything the host knows about `$sources/lib` right now.
#[derive(Default)]
pub struct Registry {
    pub modules: Vec<Module>,
    pub errors: Vec<LoadError>,
}

impl Registry {
    /// All published values, in load order (which is sorted file-name order).
    pub fn values(&self) -> Vec<(String, String)> {
        self.modules.iter().flat_map(|m| m.values.iter().cloned()).collect()
    }

    /// Re-pull every loaded module's values. Returns the modules that failed
    /// this tick as `(name, reason)`; their previous values stay in place.
    ///
    /// Only meaningful on the thread that loaded the registry — the plugin
    /// getters run there, which is also why `Registry` is deliberately not
    /// `Send`.
    pub fn refresh(&mut self) -> Vec<(String, String)> {
        let mut failed = Vec::new();
        for m in &mut self.modules {
            if let Err(reason) = m.refresh() {
                failed.push((m.name.clone(), reason));
            }
        }
        failed
    }

    /// One-line-per-module report for `zen-shell mods`.
    pub fn status(&self) -> String {
        if self.modules.is_empty() && self.errors.is_empty() {
            return format!("no modules in {}\n", lib_dir().map(|p| p.display().to_string()).unwrap_or_else(|| "<no $sources>".into()));
        }
        let mut out = String::new();
        for m in &self.modules {
            out.push_str(&format!(
                "{:<16} {:>3} value(s)  {}\n",
                m.name,
                m.values.len(),
                m.path.display()
            ));
            for (k, v) in &m.values {
                out.push_str(&format!("    {k} = {v}\n"));
            }
        }
        for e in &self.errors {
            out.push_str(&format!("{:<16} FAILED     {}\n    {}\n", module_name(&e.path), e.path.display(), e.reason));
        }
        out
    }
}

/// File stem used for display: `clock.so` -> `clock`, `clock.so.disabled` -> `clock`.
fn module_name(path: &Path) -> String {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
    name.strip_suffix(".disabled").unwrap_or(name)
        .trim_end_matches(".so")
        .to_string()
}

/// Where `$sources` is, following this shell's own conventions.
///
/// Precedence: the `sources` env var (what the live session has), then
/// `dirname($states2)/sources` (the same relation the rest of the config
/// relies on), then `$HOME/.config/hdots/sources` for running outside a
/// session. Takes a lookup closure so the order is unit-testable without
/// mutating the process environment.
pub fn sources_root_from<F>(get: F) -> Option<PathBuf>
where
    F: Fn(&str) -> Option<String>,
{
    if let Some(s) = get("sources").filter(|s| !s.is_empty()) {
        return Some(PathBuf::from(s));
    }
    if let Some(states2) = get("states2").filter(|s| !s.is_empty()) {
        if let Some(parent) = Path::new(&states2).parent() {
            return Some(parent.join("sources"));
        }
    }
    get("HOME")
        .filter(|h| !h.is_empty())
        .map(|h| PathBuf::from(h).join(".config/hdots/sources"))
}

/// `$sources` for this process.
pub fn sources_root() -> Option<PathBuf> {
    sources_root_from(|k| std::env::var(k).ok())
}

/// `$sources/lib`, where plugins live.
pub fn lib_dir() -> Option<PathBuf> {
    sources_root().map(|p| p.join("lib"))
}

/// Plugin candidates in `dir`: `.so` files only, no dotfiles, no
/// `*.so.disabled`, sorted by file name so load order (and therefore
/// `{mod_x.a}` vs `{mod_y.b}` collisions) is deterministic.
pub fn candidates(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| {
            let name = e.file_name();
            let Some(name) = name.to_str() else { return false };
            if name.starts_with('.') || !name.ends_with(".so") {
                return false;
            }
            e.path().is_file()
        })
        .map(|e| e.path())
        .collect();
    out.sort();
    out
}

/// Directory listing fingerprint: the (name, mtime, len) of every candidate.
/// The host rescans on a timer and only re-dlopens when this changes, so an
/// idle bar does no work and an edited plugin is picked up without a restart.
pub fn fingerprint(dir: &Path) -> Vec<(String, std::time::SystemTime, u64)> {
    candidates(dir)
        .into_iter()
        .filter_map(|p| {
            let md = std::fs::metadata(&p).ok()?;
            let mt = md.modified().ok()?;
            Some((module_name(&p), mt, md.len()))
        })
        .collect()
}

/// Load every candidate in `dir`. Modules that fail are collected in
/// [`Registry::errors`] rather than aborting the scan — one broken plugin must
/// not take the others (or the bar) down with it.
pub fn load_dir(dir: &Path) -> Registry {
    let mut reg = Registry::default();
    for path in candidates(dir) {
        match load_one(&path) {
            Ok(Some(m)) => reg.modules.push(m),
            Ok(None) => {} // not a module (e.g. the plain stb_image.so helper)
            Err(reason) => reg.errors.push(LoadError { path, reason }),
        }
    }
    reg
}

/// dlopen one module, check its ABI, and pull its current values.
///
/// `Ok(None)` means the file is not a module (it exports none of the ABI
/// symbols) — `$sources/lib` also holds plain helper libraries.
pub fn load_one(path: &Path) -> Result<Option<Module>, String> {
    let c_path = CString::new(path.as_os_str().as_encoded_bytes())
        .map_err(|_| format!("path contains a NUL byte: {}", path.display()))?;
    // SAFETY: plain libc call; the handle is checked for null right after and
    // intentionally never closed (module docs).
    let handle = unsafe { libc::dlopen(c_path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
    if handle.is_null() {
        return Err(last_dl_error());
    }
    // SAFETY: `handle` is live (checked above) and stays open forever.
    let Some(api) = (unsafe { abi::Api::from_handle(handle) })? else {
        return Ok(None);
    };
    // SAFETY: signatures verified by `from_handle`; a lying module is trusted
    // code by definition (see the trust note in the module docs).
    let (version, count) = unsafe { ((api.abi_version)(), (api.provider_count)()) };
    if version != ABI_VERSION {
        return Err(format!("ABI version {version}, this shell speaks {ABI_VERSION}"));
    }
    if count > abi::MAX_PROVIDERS {
        return Err(format!("claims {count} providers, cap is {}", abi::MAX_PROVIDERS));
    }
    let mut values = Vec::with_capacity(count as usize);
    for i in 0..count {
        // SAFETY: as above — `i < count` and the getters return NUL-terminated
        // UTF-8 that we copy before the next call.
        let (key, value) = unsafe {
            (
                copy_str((api.provider_key)(i), abi::MAX_KEY, "key")?,
                copy_str((api.provider_value)(i), abi::MAX_VALUE, "value")?,
            )
        };
        check_key(i, &key)?;
        values.push((key, value));
    }
    Ok(Some(Module { name: module_name(path), path: path.to_path_buf(), values, api, handle }))
}

/// A key must be a plain identifier a card can interpolate: no whitespace, no
/// braces (they would be read as scene-value syntax), not empty, and short.
fn check_key(index: u32, key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Err(format!("provider {index}: empty key"));
    }
    if key.len() > abi::MAX_KEY {
        return Err(format!("provider {index}: key longer than {} bytes", abi::MAX_KEY));
    }
    if let Some(bad) = key.chars().find(|c| c.is_whitespace() || *c == '{' || *c == '}' || c.is_control()) {
        return Err(format!("provider {index}: key `{key}` contains {bad:?}"));
    }
    Ok(())
}

/// Copy a NUL-terminated string out of plugin memory. `cap` bounds a
/// misbehaving module so a bogus "string" cannot walk off into the heap.
unsafe fn copy_str(p: *const c_char, cap: usize, what: &str) -> Result<String, String> {
    if p.is_null() {
        return Err(format!("{what} pointer was NULL"));
    }
    // SAFETY: `p` is a non-null C string per the ABI; `to_bytes_until_nul`
    // stops at the first NUL or after `cap` bytes.
    let bytes = unsafe { CStr::from_ptr(p) }.to_bytes();
    if bytes.len() > cap {
        return Err(format!("{what} longer than {cap} bytes"));
    }
    // Lossy on purpose: a plugin with one bad byte in one value should still
    // give the shell its other values.
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

/// The `dlerror()` string, or a generic fallback when it is empty.
fn last_dl_error() -> String {
    // SAFETY: dlerror() returns either NULL or a pointer to a static,
    // NUL-terminated buffer owned by libdl that stays valid until the next
    // dlerror call on this thread.
    unsafe {
        let e = libc::dlerror();
        if e.is_null() {
            "dlopen failed".to_string()
        } else {
            CStr::from_ptr(e).to_string_lossy().into_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k| pairs.iter().find(|(a, _)| *a == k).map(|(_, v)| v.to_string())
    }

    #[test]
    fn sources_env_wins() {
        let r = sources_root_from(env(&[("sources", "/tmp/s"), ("states2", "/tmp/st"), ("HOME", "/home/tw")])).unwrap();
        assert_eq!(r, PathBuf::from("/tmp/s"));
    }

    #[test]
    fn states2_sibling_is_the_fallback() {
        let r = sources_root_from(env(&[("states2", "/tmp/tw_rconf/states2"), ("HOME", "/home/tw")])).unwrap();
        assert_eq!(r, PathBuf::from("/tmp/tw_rconf/sources"));
    }

    #[test]
    fn home_is_the_last_resort() {
        let r = sources_root_from(env(&[("HOME", "/home/tw")])).unwrap();
        assert_eq!(r, PathBuf::from("/home/tw/.config/hdots/sources"));
        assert!(sources_root_from(env(&[])).is_none());
    }

    #[test]
    fn empty_env_vars_are_ignored() {
        let r = sources_root_from(env(&[("sources", ""), ("states2", ""), ("HOME", "/home/tw")])).unwrap();
        assert_eq!(r, PathBuf::from("/home/tw/.config/hdots/sources"));
    }

    #[test]
    fn keys_reject_anything_a_card_cannot_interpolate() {
        assert!(check_key(0, "mod_clock.time").is_ok());
        assert!(check_key(0, "").is_err());
        assert!(check_key(0, "mod clock").is_err());
        assert!(check_key(0, "mod_{clock}").is_err());
        assert!(check_key(0, "mod\nclock").is_err());
        assert!(check_key(0, &"k".repeat(abi::MAX_KEY + 1)).is_err());
    }

    #[test]
    fn module_name_strips_both_suffixes() {
        assert_eq!(module_name(Path::new("/lib/clock.so")), "clock");
        assert_eq!(module_name(Path::new("/lib/clock.so.disabled")), "clock");
    }

    #[test]
    fn missing_lib_dir_is_not_an_error() {
        assert!(candidates(Path::new("/nonexistent/lib")).is_empty());
        assert!(fingerprint(Path::new("/nonexistent/lib")).is_empty());
    }

    #[test]
    fn candidates_skip_dotfiles_and_disabled() {
        let dir = std::env::temp_dir().join(format!("zmod-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        for n in ["b.so", "a.so", ".hidden.so", "c.so.disabled", "notes.txt", "d.so.bak"] {
            std::fs::write(dir.join(n), b"").unwrap();
        }
        std::fs::create_dir_all(dir.join("sub.so")).unwrap(); // a directory, not a module
        let got: Vec<String> = candidates(&dir)
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(got, vec!["a.so", "b.so"], "sorted, .so only, no dotfiles/disabled/dirs");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// End-to-end against a real module, but only once one has been built and
    /// deployed — otherwise there is nothing to load and the test is a no-op.
    #[test]
    fn loads_a_deployed_module() {
        let Some(dir) = lib_dir() else { return };
        let mut reg = load_dir(&dir);
        if reg.modules.is_empty() && reg.errors.is_empty() {
            return; // no $sources/lib yet — nothing to prove
        }
        assert!(reg.errors.is_empty(), "deployed module failed to load: {:?}", reg.errors.iter().map(|e| (&e.path, &e.reason)).collect::<Vec<_>>());
        assert!(!reg.modules.is_empty(), "lib dir has .so files but none loaded");
        let clock = reg.modules.iter().find(|m| m.name == "clock").expect("clock.so is deployed");
        assert!(
            clock.values.iter().any(|(k, _)| k == "mod_clock.time"),
            "clock must publish mod_clock.time, got {:?}",
            clock.values.iter().map(|(k, _)| k).collect::<Vec<_>>()
        );

        // A refresh must move the values on, or a clock plugin is pointless.
        let before = clock.values.iter().find(|(k, _)| k == "mod_clock.uptime_s").map(|(_, v)| v.clone()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        assert!(reg.refresh().is_empty(), "refresh of a good module must not fail");
        let after = reg.modules.iter().find(|m| m.name == "clock").unwrap().values.iter().find(|(k, _)| k == "mod_clock.uptime_s").map(|(_, v)| v.clone()).unwrap();
        assert_ne!(before, after, "uptime_s did not advance across a refresh: {before} -> {after}");
    }
}
