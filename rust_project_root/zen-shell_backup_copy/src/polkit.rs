//! Polkit authentication agent — replaces mate-polkit / hyprpolkitagent.
//!
//! Registers on the D-Bus session bus as `org.freedesktop.PolicyKit1.Agent`.
//! When a privileged action (pkexec, shutdown, Wi-Fi connect, …) needs
//! authentication, polkitd calls `BeginAuthentication` on this agent.
//! The agent pushes a password dialog to the shell; the user enters their
//! password; the agent calls `AuthenticationAgentResponse` back on the
//! authority. Runs on a dedicated zbus thread (never blocks the wayland loop).

use smithay_client_toolkit::reexports::calloop;
use std::sync::{Arc, Mutex, OnceLock};
use zbus::blocking::{Connection, Proxy};
use zbus::interface;

/// Messages pushed from the zbus thread to the wayland event loop.
pub enum PolkitMsg {
    /// A new authentication request — shows the password dialog.
    BeginAuth {
        action_id: String,
        message: String,
        icon_name: String,
        cookie: String,
        user_id: u32,
    },
    /// Auth was cancelled by the authority (another agent responded, timeout, etc.).
    CancelAuth,
}

/// Commands the shell sends back to the polkit agent thread.
pub enum PolkitCmd {
    /// User submitted a password — respond to the authority.
    Response { cookie: String, uid: u32, accepted: bool },
}

/// Shared state between the zbus handler and the main thread.
struct AgentState {
    sender: calloop::channel::Sender<PolkitMsg>,
    /// Current auth session (cookie + uid) — set on BeginAuth, cleared on response.
    session: Option<(String, u32)>,
}

static STATE: OnceLock<Arc<Mutex<AgentState>>> = OnceLock::new();

/// The D-Bus object implementing `org.freedesktop.PolicyKit1.Agent`.
struct PolkitAgent;

#[interface(name = "org.freedesktop.PolicyKit1.Agent")]
impl PolkitAgent {
    /// Called by polkitd when a privileged action needs authentication.
    /// Runs on zbus's executor thread — never block, never do round-trips.
    fn begin_authentication(
        &mut self,
        action_id: &str,
        message: &str,
        icon_name: &str,
        details: std::collections::HashMap<String, String>,
        cookie: &str,
        user_id: u32,
        #[zbus(header)] _header: zbus::message::Header<'_>,
    ) {
        let _ = details; // we don't need the details dict
        if let Some(state) = STATE.get() {
            let mut s = state.lock().unwrap();
            s.session = Some((cookie.to_string(), user_id));
            let _ = s.sender.send(PolkitMsg::BeginAuth {
                action_id: action_id.to_string(),
                message: message.to_string(),
                icon_name: icon_name.to_string(),
                cookie: cookie.to_string(),
                user_id,
            });
        }
    }

    /// Called by polkitd to cancel an in-progress authentication.
    fn cancel_authentication(&mut self) {
        if let Some(state) = STATE.get() {
            let mut s = state.lock().unwrap();
            s.session = None;
            let _ = s.sender.send(PolkitMsg::CancelAuth);
        }
    }

    /// List temporary authorizations — we don't track any.
    fn enumerate_temporary_authorizations(
        &self,
    ) -> Vec<(String, String, (String, std::collections::HashMap<String, String>), u64, u64)> {
        Vec::new()
    }

    /// A temporary authorization expired — no-op for us.
    fn temporary_authorization_expired(
        &mut self,
        _id: &str,
        _subject: (String, std::collections::HashMap<String, String>),
        _action_id: &str,
        _details: std::collections::HashMap<String, String>,
        _temporary_authorization: (String, std::collections::HashMap<String, String>),
    ) {
    }
}

/// Spawn the polkit agent daemon thread. Returns the thread handle and
/// the command sender (shell sends `PolkitCmd::Response` after the user
/// enters their password).
pub fn spawn(
    sender: calloop::channel::Sender<PolkitMsg>,
) -> Option<(
    std::thread::JoinHandle<()>,
    std::sync::mpsc::Sender<PolkitCmd>,
)> {
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();
    let h = std::thread::Builder::new()
        .name("zen-polkit".into())
        .spawn(move || {
            if let Err(e) = run(sender, cmd_rx) {
                eprintln!("zen: polkit agent: {e}");
            }
        })
        .ok()?;
    Some((h, cmd_tx))
}

fn run(
    sender: calloop::channel::Sender<PolkitMsg>,
    cmd_rx: std::sync::mpsc::Receiver<PolkitCmd>,
) -> zbus::Result<()> {
    use zbus::blocking::connection::Builder as ZBuilder;

    let state = Arc::new(Mutex::new(AgentState {
        sender,
        session: None,
    }));
    let _ = STATE.set(state);

    let agent = PolkitAgent;
    let conn = ZBuilder::session()?
        .serve_at("/org/freedesktop/PolicyKit1/Agent", agent)?
        .build()?;

    // Register with the polkit authority on the system bus.
    // Subject = (bus_name, { "unix-user": uid }) — identifies this session.
    register_with_authority(&conn)?;

    eprintln!("zen: polkit agent registered");

    // Process response commands from the shell.
    loop {
        match cmd_rx.recv() {
            Ok(PolkitCmd::Response { cookie, uid, accepted }) => {
                respond_to_authority(&cookie, uid, accepted);
                if let Some(s) = STATE.get() {
                    s.lock().unwrap().session = None;
                }
            }
            Err(_) => break,
        }
    }
    Ok(())
}

/// Register this agent with polkitd on the system bus.
fn register_with_authority(_session_conn: &Connection) -> zbus::Result<()> {
    let system_conn = Connection::system()?;

    // Subject: (unix-session, { "session-id": <logind session> }).
    // The agent object is served on the SESSION bus, but `our_name` above
    // is a session-bus name — polkitd runs on the system bus and would fail
    // to resolve it ("Unknown subject of kind ':1.1910'"), killing the
    // agent thread. polkitd resolves the `unix-session` kind through logind
    // and reaches the agent on the session's bus, which is where we serve.
    let uid = unsafe { libc::getuid() } as u32;
    let session_id =
        std::env::var("XDG_SESSION_ID").unwrap_or_else(|_| String::new());
    if session_id.is_empty() {
        return Err(zbus::Error::Failure(
            "XDG_SESSION_ID not set — cannot register polkit agent".into(),
        ));
    }
    let mut subject_props = std::collections::HashMap::new();
    subject_props.insert(
        "unix-user".to_string(),
        zbus::zvariant::Value::from(uid),
    );
    subject_props.insert(
        "session-id".to_string(),
        zbus::zvariant::Value::from(session_id),
    );
    let subject: (String, std::collections::HashMap<String, zbus::zvariant::Value>) =
        ("unix-session".to_string(), subject_props);

    let proxy = Proxy::new(
        &system_conn,
        "org.freedesktop.PolicyKit1",
        "/org/freedesktop/PolicyKit1/Authority",
        "org.freedesktop.PolicyKit1.Authority",
    )?;

    let locale = std::env::var("LANG")
        .or_else(|_| std::env::var("LC_ALL"))
        .unwrap_or_else(|_| "en_US.UTF-8".into());

    proxy.call_method(
        "RegisterAuthenticationAgent",
        &(
            subject,
            locale,
            "/org/freedesktop/PolicyKit1/Agent",
        ),
    )?;

    Ok(())
}

/// Respond to the polkit authority with the user's password decision.
fn respond_to_authority(cookie: &str, uid: u32, accepted: bool) {
    let Ok(system_conn) = Connection::system() else {
        eprintln!("zen: polkit: cannot connect to system bus");
        return;
    };

    let proxy = match Proxy::new(
        &system_conn,
        "org.freedesktop.PolicyKit1",
        "/org/freedesktop/PolicyKit1/Authority",
        "org.freedesktop.PolicyKit1.Authority",
    ) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("zen: polkit: proxy: {e}");
            return;
        }
    };

    if accepted {
        // Build the identity: (user_bus_name, { "unix-user": uid })
        // The identity must match the user being authenticated.
        let mut identity_props = std::collections::HashMap::new();
        identity_props.insert(
            "unix-user".to_string(),
            zbus::zvariant::Value::from(uid),
        );
        let identity: (
            String,
            std::collections::HashMap<String, zbus::zvariant::Value>,
        ) = ("".to_string(), identity_props);

        // Use Response3 (cookie, identity, subject) for polkit 121+
        // Fall back to Response (cookie, identity) for older versions
        let result = proxy.call_method(
            "AuthenticationAgentResponse3",
            &(cookie.to_string(), identity.clone(), identity),
        );
        if let Err(_e) = result {
            // Fallback: try the older Response method
            let mut identity_props = std::collections::HashMap::new();
            identity_props.insert(
                "unix-user".to_string(),
                zbus::zvariant::Value::from(uid),
            );
            let identity: (
                String,
                std::collections::HashMap<String, zbus::zvariant::Value>,
            ) = ("".to_string(), identity_props);

            let _ = proxy.call_method(
                "AuthenticationAgentResponse",
                &(cookie.to_string(), identity),
            );
        }
    } else {
        // User cancelled — we don't call Response, polkitd will deny the action.
        // But some polkit versions expect a Response with a rejected identity.
        // The safest approach: just don't respond. polkitd times out and denies.
    }
}

/// Verify a password against PAM (runs on a worker thread).
/// Returns true if the password is correct.
pub fn verify_password(password: &str) -> bool {
    use std::io::Write;

    // Simple PAM check via pam_authenticate.
    // We fork a process that uses `su -c "true" <user>` as a proxy —
    // if it succeeds, the password is correct.
    let user = std::env::var("USER").unwrap_or_else(|_| "root".into());

    let child = std::process::Command::new("su")
        .arg("-c")
        .arg("true")
        .arg(&user)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    match child {
        Ok(mut proc) => {
            if let Some(ref mut stdin) = proc.stdin {
                let _ = stdin.write_all(password.as_bytes());
                let _ = stdin.write_all(b"\n");
            }
            proc.wait()
                .map(|status| status.success())
                .unwrap_or(false)
        }
        Err(e) => {
            eprintln!("zen: polkit: su failed: {e}");
            false
        }
    }
}
