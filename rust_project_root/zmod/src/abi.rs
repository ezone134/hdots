//! Raw ABI surface of a data-provider plugin.
//!
//! Mirrors `include/zenmod.h`. Everything here is `unsafe` glue: the four
//! function pointers below are resolved with `dlsym` and are only ever called
//! on a handle that `dlopen`ed successfully and reported a matching ABI
//! version. The safe wrappers live in the parent module.

use std::ffi::{c_char, c_void};

/// Must match `ZEN_ABI_VERSION` in `include/zenmod.h`.
pub const ABI_VERSION: u32 = 1;

/// Sanity caps, mirrored from the header. A plugin that reports more slots
/// than this is broken, not a feature.
pub const MAX_PROVIDERS: u32 = 4096;
pub const MAX_KEY: usize = 128;
pub const MAX_VALUE: usize = 65536;

pub type AbiVersionFn = unsafe extern "C" fn() -> u32;
pub type ProviderCountFn = unsafe extern "C" fn() -> u32;
pub type ProviderKeyFn = unsafe extern "C" fn(i: u32) -> *const c_char;
pub type ProviderValueFn = unsafe extern "C" fn(i: u32) -> *const c_char;

/// The four symbols a module must export, with the types to call them through.
#[derive(Clone, Copy)]
pub struct Api {
    pub abi_version: AbiVersionFn,
    pub provider_count: ProviderCountFn,
    pub provider_key: ProviderKeyFn,
    pub provider_value: ProviderValueFn,
}

impl Api {
    /// Resolve the ABI out of an open `dlopen` handle.
    ///
    /// Returns `Ok(None)` when the object exports none of the four symbols:
    /// `$sources/lib` is also where plain helper libraries live (the shell's
    /// own `stb_image.so` is one), and those are not modules. A module that
    /// exports *some* of the symbols is a broken module and gets an error.
    ///
    /// # Safety
    /// `handle` must be a live handle returned by `dlopen` that is not closed
    /// while the returned `Api` is in use — the shell never calls `dlclose`,
    /// which is exactly what makes this sound.
    pub unsafe fn from_handle(handle: *mut c_void) -> Result<Option<Api>, String> {
        let sym = |name: &str| -> *mut c_void {
            let Ok(c) = std::ffi::CString::new(name) else { return std::ptr::null_mut() };
            libc::dlsym(handle, c.as_ptr())
        };
        let (v, c, k, val) = (
            sym("zen_abi_version"),
            sym("zen_provider_count"),
            sym("zen_provider_key"),
            sym("zen_provider_value"),
        );
        if v.is_null() && c.is_null() && k.is_null() && val.is_null() {
            return Ok(None); // not one of ours
        }
        for (name, p) in [
            ("zen_abi_version", v),
            ("zen_provider_count", c),
            ("zen_provider_key", k),
            ("zen_provider_value", val),
        ] {
            if p.is_null() {
                return Err(format!("missing symbol `{name}`"));
            }
        }
        // SAFETY: each symbol was checked non-null above and is transmuted to
        // the signature declared in zenmod.h; a module that lies about them
        // is the same trust level as any other code run from $sources/lib.
        unsafe {
            Ok(Some(Api {
                abi_version: std::mem::transmute::<*mut c_void, AbiVersionFn>(v),
                provider_count: std::mem::transmute::<*mut c_void, ProviderCountFn>(c),
                provider_key: std::mem::transmute::<*mut c_void, ProviderKeyFn>(k),
                provider_value: std::mem::transmute::<*mut c_void, ProviderValueFn>(val),
            }))
        }
    }
}
