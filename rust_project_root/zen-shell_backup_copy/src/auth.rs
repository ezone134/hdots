//! Lockscreen password check — minimal PAM client.
//!
//! Loads `libpam.so.0` at runtime (dlopen), so building needs no `pam-devel`
//! headers. `verify_password` runs `pam_authenticate` against the system's
//! PAM stack (tries `zen-shell` / `swaylock` / `hyprlock` / `login` /
//! `system-auth` / `common-auth`, whichever service file exists), which is the
//! same mechanism swaylock / hyprlock use.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::sync::OnceLock;

const PAM_PROMPT_ECHO_OFF: c_int = 1;
const PAM_SUCCESS: c_int = 0;
const PAM_SYSTEM_ERR: c_int = 9;

#[repr(C)]
struct PamMessage {
    msg_style: c_int,
    msg: *const c_char,
}

#[repr(C)]
struct PamResponse {
    resp: *mut c_char,
    resp_retcode: c_int,
}

type ConvFn = unsafe extern "C" fn(
    num_msg: c_int,
    msg: *const *const PamMessage,
    resp: *mut *mut PamResponse,
    appdata: *mut c_void,
) -> c_int;

#[repr(C)]
struct PamConv {
    conv: Option<ConvFn>,
    appdata: *mut c_void,
}

type PamStart = unsafe extern "C" fn(
    service: *const c_char,
    user: *const c_char,
    conv: *const PamConv,
    pamh: *mut *mut c_void,
) -> c_int;
type PamAuthenticate = unsafe extern "C" fn(pamh: *mut c_void, flags: c_int) -> c_int;
type PamEnd = unsafe extern "C" fn(pamh: *mut c_void, status: c_int) -> c_int;

struct PamFns {
    start: PamStart,
    authenticate: PamAuthenticate,
    end: PamEnd,
}

static PAM: OnceLock<Option<PamFns>> = OnceLock::new();

fn pam_fns() -> Option<&'static PamFns> {
    PAM.get_or_init(|| unsafe {
        let lib = libc::dlopen(c"libpam.so.0".as_ptr(), libc::RTLD_NOW);
        if lib.is_null() {
            return None;
        }
        let sym = |name: &[u8]| libc::dlsym(lib, name.as_ptr() as *const c_char);
        let start = sym(b"pam_start\0");
        let authenticate = sym(b"pam_authenticate\0");
        let end = sym(b"pam_end\0");
        if start.is_null() || authenticate.is_null() || end.is_null() {
            return None;
        }
        Some(PamFns {
            start: std::mem::transmute(start),
            authenticate: std::mem::transmute(authenticate),
            end: std::mem::transmute(end),
        })
    })
    .as_ref()
}

/// PAM conversation: answer the echo-off (password) prompt with the password
/// in `appdata`; every other prompt gets an empty response. PAM frees the
/// returned strings with `free()`, so they must be malloc'd (strdup).
unsafe extern "C" fn conv(
    num_msg: c_int,
    msgs: *const *const PamMessage,
    resp_out: *mut *mut PamResponse,
    appdata: *mut c_void,
) -> c_int {
    if num_msg <= 0 {
        return PAM_SUCCESS;
    }
    let resp = libc::calloc(num_msg as usize, std::mem::size_of::<PamResponse>()) as *mut PamResponse;
    if resp.is_null() {
        return PAM_SYSTEM_ERR;
    }
    for i in 0..num_msg as isize {
        let m = *msgs.offset(i);
        (*resp.offset(i)).resp_retcode = 0;
        (*resp.offset(i)).resp = if (*m).msg_style == PAM_PROMPT_ECHO_OFF {
            libc::strdup(appdata as *const c_char)
        } else {
            std::ptr::null_mut()
        };
    }
    *resp_out = resp;
    PAM_SUCCESS
}

/// The current user, from `$USER` or the passwd database.
pub fn current_user() -> String {
    if let Ok(u) = std::env::var("USER") {
        if !u.is_empty() {
            return u;
        }
    }
    unsafe {
        let uid = libc::geteuid();
        let mut buf = [0i8; 256];
        let mut pwd: libc::passwd = std::mem::zeroed();
        let mut res: *mut libc::passwd = std::ptr::null_mut();
        if libc::getpwuid_r(uid, &mut pwd, buf.as_mut_ptr(), buf.len(), &mut res) == 0
            && !res.is_null()
            && !pwd.pw_name.is_null()
        {
            return CStr::from_ptr(pwd.pw_name).to_string_lossy().into_owned();
        }
    }
    "user".into()
}

/// Verify `password` for `user` through PAM. `Ok(true)` = correct,
/// `Ok(false)` = wrong password, `Err` = PAM unavailable (no libpam / no
/// service file) — callers must decide how to handle that.
///
/// Denials log the PAM error code to stderr so a real "wrong password"
/// (PAM_AUTH_ERR = 7) is distinguishable from a broken stack (faillock
/// lockout, missing service) when diagnosing the lockscreen.
pub fn verify_password(user: &str, password: &str) -> Result<bool, String> {
    let Some(fns) = pam_fns() else {
        return Err("libpam.so.0 not found".into());
    };
    let user = CString::new(user).map_err(|_| "invalid user".to_string())?;
    let pw = CString::new(password).map_err(|_| "invalid password".to_string())?;
    let conv = PamConv {
        conv: Some(conv),
        appdata: pw.as_ptr() as *mut c_void,
    };
    // hyprlock/swaylock-style fallback chain: the distro's lock services first,
    // then generic login stacks. `login` exists nearly everywhere and routes
    // through system-auth, so this covers stock Arch/Fedora/Debian.
    for service in [
        "zen-shell",
        "hyprlock",
        "swaylock",
        "login",
        "system-auth",
        "common-auth",
        "system-local-login",
        "su",
        "passwd",
    ] {
        let Ok(svc) = CString::new(service) else { continue };
        let mut pamh: *mut c_void = std::ptr::null_mut();
        let rc = unsafe { (fns.start)(svc.as_ptr(), user.as_ptr(), &conv, &mut pamh) };
        if rc != PAM_SUCCESS {
            continue; // no such service — try the next
        }
        let auth = unsafe { (fns.authenticate)(pamh, 0) };
        unsafe { (fns.end)(pamh, auth) };
        if auth == PAM_SUCCESS {
            return Ok(true);
        }
        // log the denial — wrong password (7) vs a broken stack (faillock
        // lockout 9, account expiry 6, …) look identical in the UI
        eprintln!("zen: pam authenticate denied via '{service}' (rc={auth})");
        return Ok(false);
    }
    Err("no usable PAM service".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A wrong password must come back as `Ok(false)` (PAM loaded and ran),
    /// not `Err` (which would mean libpam/service missing).
    #[test]
    fn wrong_password_is_rejected_not_unavailable() {
        let user = current_user();
        match verify_password(&user, "zen-shell-test-wrong-password") {
            Ok(ok) => assert!(!ok, "a wrong password must not verify"),
            Err(e) => panic!("PAM unavailable on this system: {e}"),
        }
    }
}
