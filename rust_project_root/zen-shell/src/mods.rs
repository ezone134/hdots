//! Data-provider modules from `$sources/lib/*.so`.
//!
//! A module is a shared object exporting the four functions in
//! `zmod/include/zenmod.h`; it publishes `(key, value)` pairs that land in the
//! same namespace as the shell's built-in scene values, so any `.ron` card can
//! bind them with `{mod_clock.time}`-style interpolation.
//!
//! Shape of the thing, and why:
//!
//!  * **A worker thread owns the registry.** `zmod::Registry` is deliberately
//!    not `Send` (it holds a `dlopen` handle), and the plugin getters run
//!    there. The render thread only ever sees an `Arc<Snapshot>` of plain
//!    `String`s behind an `RwLock`, so a slow or misbehaving module can never
//!    stall a frame.
//!  * **A re-`dlopen` only happens when the file set changes.** Every tick
//!    re-pulls *values* from the already-open modules (that is what makes a
//!    clock tick); the `dlopen`/`dlsym` path runs only when
//!    `zmod::fingerprint` changes, i.e. a module was added, removed or
//!    rewritten. Rewriting a live `.so` in place keeps working because the old
//!    inode stays mapped — copy the new build to a new name (or restart the
//!    bar) to actually pick it up.
//!  * **Failures are data, not panics.** A module that fails to load or throws
//!    on a tick is reported by `zen-shell mods`; it never takes the bar down.

use std::sync::{Arc, OnceLock, RwLock};
use std::time::Duration;

/// How often values are re-pulled. One second matches the shell's own clock
/// and keeps the cost at one pass over a handful of short strings.
const TICK: Duration = Duration::from_secs(1);

/// What the render thread gets to see.
#[derive(Default, Clone)]
pub struct Snapshot {
    /// Every module's values, in sorted-load order.
    pub values: Vec<(String, String)>,
    /// Human-readable report for `zen-shell mods` (modules, keys, failures).
    pub status: String,
}

/// Shared snapshot slot. `OnceLock` + `RwLock` because the worker publishes
/// whole snapshots (cheap: an `Arc` swap) and readers only clone an `Arc`.
static SNAPSHOT: OnceLock<RwLock<Arc<Snapshot>>> = OnceLock::new();

fn slot() -> &'static RwLock<Arc<Snapshot>> {
    SNAPSHOT.get_or_init(|| {
        // Seeded rather than left empty so `zen-shell mods` says something
        // useful during the first second, before the worker's first tick.
        let status = match zmod::lib_dir() {
            Some(dir) => format!("scanning {} ...\n", dir.display()),
            None => "no $sources: modules unavailable\n".to_string(),
        };
        RwLock::new(Arc::new(Snapshot { values: Vec::new(), status }))
    })
}

/// Spawn the module thread. Idempotent, and safe to call before the first
/// frame: `values()` works (and returns nothing) even if it never ran.
pub fn start() {
    static STARTED: OnceLock<()> = OnceLock::new();
    if STARTED.set(()).is_err() {
        return;
    }
    if std::thread::Builder::new()
        .name("zen-mods".into())
        .spawn(worker)
        .is_err()
    {
        eprintln!("zen: could not spawn the module thread; $sources/lib modules are unavailable");
    }
}

/// Latest published values, cloned for the frame that is being built.
pub fn values() -> Vec<(String, String)> {
    match slot().read() {
        Ok(s) => s.values.clone(),
        // A poisoned lock means the worker panicked; the shell keeps running
        // without module values rather than propagating someone else's panic.
        Err(_) => Vec::new(),
    }
}

/// Report for `zen-shell mods`.
pub fn status() -> String {
    match slot().read() {
        Ok(s) => s.status.clone(),
        Err(_) => "module thread state is unavailable\n".to_string(),
    }
}

/// The worker loop: owns the registry, republishes a snapshot every tick.
fn worker() {
    // Defensive, like the viz thread: a panic here must not take the bar with
    // it. `Registry` is rebuilt from scratch below, so bailing out on a panic
    // only costs the module values until the next restart.
    let _ = std::panic::catch_unwind(|| {
        let dir = zmod::lib_dir();
        let mut registry: Option<zmod::Registry> = None;
        let mut fingerprint: Vec<(String, std::time::SystemTime, u64)> = Vec::new();
        loop {
            if let Some(dir) = dir.clone() {
                let fp = zmod::fingerprint(&dir);
                let failed: Vec<(String, String)>;
                match registry.take() {
                    // Same files as last tick: keep the open modules and just
                    // re-pull their values — this is the cheap path, and the
                    // only one a running bar normally takes.
                    Some(mut reg) if fp == fingerprint => {
                        failed = reg.refresh();
                        registry = Some(reg);
                    }
                    // File set changed (or first tick): load from scratch. The
                    // old handles are still mapped and simply abandoned.
                    _ => {
                        let reg = zmod::load_dir(&dir);
                        fingerprint = fp;
                        failed = reg
                            .errors
                            .iter()
                            .map(|e| (e.path.display().to_string(), e.reason.clone()))
                            .collect();
                        registry = Some(reg);
                    }
                }
                publish(registry.as_ref(), &failed);
            }
            std::thread::sleep(TICK);
        }
    });
}

/// Swap in a new snapshot. `failed` is this tick's per-module problems, which
/// are transient by nature (a load error is sticky, a refresh error is not).
fn publish(registry: Option<&zmod::Registry>, failed: &[(String, String)]) {
    let (values, mut status) = match registry {
        Some(reg) => (reg.values(), reg.status()),
        None => (Vec::new(), "no $sources: modules unavailable\n".to_string()),
    };
    for (name, reason) in failed {
        status.push_str(&format!("{name:<16} this tick   {reason}\n"));
    }
    if let Ok(mut s) = slot().write() {
        *s = Arc::new(Snapshot { values, status });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // NB: there is deliberately no "values() is empty before start()" test —
    // `start()` is process-global and OnceLock-guarded, so a sibling test that
    // starts the worker would race it. The empty case is covered
    // deterministically by `merge_module_values` with an empty vec in
    // shell::panels.

    /// `zen-shell mods` always says something useful, even with no
    /// `$sources/lib` at all.
    #[test]
    fn status_is_readable_without_modules() {
        let s = status();
        assert!(!s.is_empty(), "status must never be empty");
        assert!(s.contains("sources") || s.contains("lib"), "status should name the lib dir, got {s:?}");
    }

    /// The whole path, against whatever is actually deployed in
    /// `$sources/lib`: start the worker, let it tick, read the values the
    /// render thread would see. Skips (rather than fails) on a machine with no
    /// modules installed — the same convention `zmod`'s own test uses.
    #[test]
    fn the_worker_publishes_deployed_module_values() {
        start();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let got = values();
            if !got.is_empty() {
                // Dotted keys survive the trip through the snapshot, and a
                // clock module's value is not the empty string.
                let (k, v) = &got[0];
                assert!(k.contains('.'), "module key should be dotted, got {k:?}");
                assert!(!v.is_empty(), "module {k} published an empty value");
                return;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "no module values after 5s; status was:\n{}",
                status()
            );
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    /// The snapshot the render thread reads is a plain Arc of Strings — no
    /// handles, no plugin pointers, nothing that could outlive a module.
    #[test]
    fn published_values_are_plain_owned_data() {
        publish(None, &[]);
        let snap: Arc<Snapshot> = Arc::clone(&slot().read().unwrap());
        assert!(snap.values.is_empty());
        assert!(snap.status.contains("$sources"));
    }
}
