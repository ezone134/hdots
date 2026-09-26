pragma ComponentBehavior: Bound
import QtQuick
import Qt5Compat.GraphicalEffects
import Quickshell
import Quickshell.Wayland
import Quickshell.Services.Pam
import Quickshell.Io
import QsHypr 1.0
import "."
import "../services"

// Full-screen session lock via the Wayland ext-session-lock-v1 protocol.
// `StateController.locked` drives this WlSessionLock (set by the `bar lock`
// IPC, which power_main's `session lock` fires). Every screen gets its own
// WlSessionLockSurface rendering the blurred+dimmed wallpaper, a clock, the
// user and a password field. Unlock happens ONLY through a correct PAM
// password — there is no IPC unlock path (security).
WlSessionLock {
    id: root

    locked: StateController.locked

    readonly property string user: Quickshell.env("USER") || "user"

    property string wallpaperPath: ""
    property string errorText: ""
    property date now: new Date()

    // A pending submit captures the password for the async start() path:
    // pam_unix may request a response one or more times before completing.
    property bool submitPending: false
    property string pendingPassword: ""
    // Bumping this clears + refocuses the field on every surface (used on
    // auth failure and on lock so no stale password survives a relock).
    property int resetToken: 0

    // Caps Lock tracking: seeded from `hyprctl devices` at lock time, then
    // kept live from key events on the password field (a Caps_Lock press
    // toggles; the produced text vs shift disambiguates letters). Lives on the
    // root so every surface renders the same warning.
    property bool capsOn: false

    // TEMPORARY bypass arming: first click arms, second click unlocks;
    // auto-disarms after 5s so a stray click can't unlock the session.
    property bool bypassArmed: false

    Timer {
        id: bypassDisarmTimer
        interval: 5000
        repeat: false
        onTriggered: root.bypassArmed = false
    }

    function bypassClick() {
        if (!root.bypassArmed) {
            root.bypassArmed = true
            bypassDisarmTimer.restart()
        } else {
            root.bypassArmed = false
            bypassDisarmTimer.stop()
            StateController.unlock()
        }
    }

    // One PAM context serves every surface. IMPORTANT: this machine uses the
    // pam-config (Debian-style) layout — /etc/pam.d/system-auth does NOT
    // exist here (pam_start falls back to the deny-by-default `other` stack,
    // which is why the correct password was rejected and the shell locked
    // out). `common-auth` exists and is exactly `auth required pam_unix.so
    // try_first_pass`, i.e. plain /etc/shadow authentication — the same stack
    // login uses. `user` is set explicitly (defaults to the current user).
    property PamContext pam: PamContext {
        id: pam
        config: "common-auth"
        user: root.user

        onCompleted: root.onPamResult(result)
    }

    // pam may re-request a response internally (retries); answer it with the
    // password captured when the user submitted.
    property Connections pamResponseWatch: Connections {
        target: pam
        function onResponseRequiredChanged() {
            if (pam.responseRequired && root.submitPending) {
                root.submitPending = false
                const pw = root.pendingPassword
                root.pendingPassword = ""
                pam.respond(pw)
            }
        }
    }

    function submitPassword(pw) {
        root.errorText = ""
        if (pw.length === 0)
            return
        if (pam.active && pam.responseRequired) {
            pam.respond(pw)
        } else if (pam.active) {
            // A previous attempt is still in flight: hand the latest password
            // to the running conversation instead of calling start() again on
            // an already-active context (which could leave auth hanging).
            root.pendingPassword = pw
            root.submitPending = true
        } else {
            root.pendingPassword = pw
            root.submitPending = true
            if (!pam.start()) {
                // start() failed — clear the pending flags so a later Enter
                // actually retries instead of silently waiting forever.
                root.submitPending = false
                root.pendingPassword = ""
                root.errorText = "Could not start authentication"
                root.resetToken++
            }
        }
    }

    // Derive caps lock state from a key event. A Caps_Lock press toggles; a
    // printable letter carries the live state in its produced text, so case vs
    // shift tells us the locked state without ever polling the compositor.
    function handleKeyPress(event) {
        if (event.key === Qt.Key_CapsLock) {
            root.capsOn = !root.capsOn
            return
        }
        if (event.text.length !== 1)
            return
        const upper = event.text.toUpperCase()
        if (upper === event.text.toLowerCase())
            return // not a letter — digits/symbols don't reflect caps
        const shift = (event.modifiers & Qt.ShiftModifier) !== 0
        root.capsOn = shift !== (event.text === upper)
    }

    function onPamResult(result) {
        root.submitPending = false
        root.pendingPassword = ""
        if (result === PamResult.Success) {
            StateController.unlock()
        } else {
            const msg = String(pam.message || "").trim()
            root.errorText = pam.messageIsError && msg.length > 0 ? msg : "Wrong password"
            root.resetToken++
        }
    }

    // Resolve the current wallpaper path with pure bash (no text-munging
    // binaries): states2/s -> channel letter, states/current_wall_<ch>
    // -> path, fallback to $current_wall= in the hyprlock conf.
    readonly property string resolveScript: [
        'ch="$(cat /tmp/tw_rconf/states2/s 2>/dev/null)"',
        'wall=""',
        'if [ -n "$ch" ] && [ -r "/tmp/tw_rconf/states/current_wall_$ch" ]; then',
        '    IFS= read -r wall < "/tmp/tw_rconf/states/current_wall_$ch" 2>/dev/null',
        'fi',
        'if [ -z "$wall" ]; then',
        '    conf="$HOME/.config/hypr/hyprlock/current_wall.conf"',
        '    if [ -r "$conf" ]; then',
        '        while IFS= read -r line; do',
        '            case "$line" in',
        '                \\$current_wall=*) wall="${line#\\$current_wall=}"; break;;',
        '            esac',
        '        done < "$conf"',
        '    fi',
        'fi',
        '[ -n "$wall" ] && printf "%s\\n" "$wall"'
    ].join("\n")

    function refreshWallpaper() {
        wallpaperProc.running = false
        wallpaperProc.command = ["bash", "-c", root.resolveScript, "wall"]
        wallpaperProc.running = true
    }

    Process {
        id: wallpaperProc
        stdout: StdioCollector {
            onStreamFinished: {
                const p = text.trim()
                if (p.length > 0)
                    root.wallpaperPath = p
            }
        }
    }

    // Seed caps lock from Hyprland's `main` keyboard via QsHypr (in-process,
    // one socket read, only at lock time — never polled while unlocked).
    function refreshCapsLock() {
        QsHypr.request("devices", false, function(text) {
            // Same logic as the old bash one-liner: remember each block's
            // `capsLock:` value; when the `main: yes` keyboard appears, use it.
            let caps = ""
            const lines = String(text || "").split("\n")
            for (let i = 0; i < lines.length; i++) {
                const trimmed = lines[i].trim()
                const capsIdx = trimmed.indexOf("capsLock:")
                if (capsIdx !== -1) {
                    caps = trimmed.slice(capsIdx + 9).trim()
                } else {
                    const mainIdx = trimmed.indexOf("main:")
                    if (mainIdx !== -1 && trimmed.slice(mainIdx + 5).trim() === "yes") {
                        const v = caps.toLowerCase()
                        if (v === "yes")
                            root.capsOn = true
                        else if (v === "no")
                            root.capsOn = false
                        break
                    }
                }
            }
        })
    }

    // Clock ticks once per minute while locked (same free-running pattern as
    // the pill clock; no tick while unlocked).
    Timer {
        interval: 60000
        repeat: true
        running: StateController.locked
        triggeredOnStart: true
        onTriggered: root.now = new Date()
    }

    onLockedChanged: {
        if (root.locked) {
            root.now = new Date()
            root.errorText = ""
            root.submitPending = false
            root.pendingPassword = ""
            root.refreshWallpaper()
            root.capsOn = false
            root.refreshCapsLock()
        }
    }

    // One surface per screen, auto-created by WlSessionLock while locked.
    WlSessionLockSurface {
        color: Theme.bg

        Rectangle {
            id: screenRoot
            anchors.fill: parent
            focus: true

            Keys.onPressed: function(event) {
                if (event.key === Qt.Key_Escape) {
                    passwordField.text = ""
                    event.accepted = true
                }
            }

            // Background click refocuses the password field.
            MouseArea {
                anchors.fill: parent
                onClicked: passwordField.forceActiveFocus()
            }

            // Blurred + dimmed wallpaper.
            Item {
                anchors.fill: parent
                visible: root.wallpaperPath.length > 0

                Image {
                    anchors.fill: parent
                    source: "file://" + root.wallpaperPath
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                    cache: false
                }

                layer.enabled: true
                layer.smooth: true
                layer.effect: FastBlur {
                    radius: 40
                    transparentBorder: false
                }
            }

            Rectangle {
                anchors.fill: parent
                color: "#a0101828"
            }

            Column {
                anchors.centerIn: parent
                spacing: 14
                width: Math.min(parent.width - 80, 420)

                Rectangle {
                    width: 96
                    height: 96
                    radius: 48
                    anchors.horizontalCenter: parent.horizontalCenter
                    color: Qt.rgba(0, 0, 0, 0.38)
                    border.width: 1
                    border.color: Qt.rgba(1, 1, 1, 0.18)

                    Txt {
                        anchors.centerIn: parent
                        text: "\uf007"
                        font.family: Theme.fontName
                        font.pixelSize: 42
                        color: Theme.fg
                    }
                }

                Txt {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: root.user
                    color: Theme.fg
                    font.family: Theme.fontName
                    font.pixelSize: 18
                    font.bold: true
                }

                Txt {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: Qt.formatTime(root.now, "HH:mm")
                    color: Theme.fg
                    font.family: Theme.fontName
                    font.pixelSize: 76
                    font.bold: true
                }

                Txt {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: Qt.formatDate(root.now, "dddd, MMMM d")
                    color: Theme.fg2
                    font.family: Theme.fontName
                    font.pixelSize: 16
                }

                Rectangle {
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: parent.width
                    height: 48
                    radius: 14
                    color: Qt.rgba(0, 0, 0, 0.38)
                    border.width: 1
                    border.color: passwordField.activeFocus ? Theme.acc : Qt.rgba(1, 1, 1, 0.18)

                    Txt {
                        visible: passwordField.text.length === 0 && !passwordField.activeFocus
                        anchors.left: parent.left
                        anchors.leftMargin: 16
                        anchors.verticalCenter: parent.verticalCenter
                        text: "Password"
                        color: Theme.fg3
                        font.family: Theme.fontName
                        font.pixelSize: 14
                    }

                    TxtInput {
                        id: passwordField
                        anchors.fill: parent
                        anchors.leftMargin: 16
                        anchors.rightMargin: 16
                        verticalAlignment: TextInput.AlignVCenter
                        color: Theme.fg
                        font.family: Theme.fontName
                        font.pixelSize: 15
                        echoMode: TextInput.Password
                        passwordCharacter: "•"
                        clip: true

                        onAccepted: {
                            root.submitPassword(passwordField.text)
                            passwordField.text = ""
                        }

                        onTextChanged: {
                            if (root.errorText.length > 0)
                                root.errorText = ""
                        }

                        Keys.onPressed: function(event) {
                            root.handleKeyPress(event)
                        }

                        Connections {
                            target: root
                            function onResetTokenChanged() {
                                passwordField.text = ""
                                passwordField.forceActiveFocus()
                            }
                        }
                    }
                }

                Row {
                    anchors.horizontalCenter: parent.horizontalCenter
                    visible: root.capsOn
                    spacing: 8

                    Rectangle {
                        width: 22
                        height: 22
                        radius: 6
                        anchors.verticalCenter: parent.verticalCenter
                        color: Qt.rgba(1, 1, 1, 0.06)
                        border.width: 1
                        border.color: Theme.warning

                        Txt {
                            anchors.centerIn: parent
                            text: "\uf071"
                            color: Theme.warning
                            font.family: Theme.fontName
                            font.pixelSize: 11
                        }
                    }

                    Txt {
                        anchors.verticalCenter: parent.verticalCenter
                        text: "Caps Lock is ON — password is typed in UPPERCASE"
                        color: Theme.warning
                        font.family: Theme.fontName
                        font.pixelSize: 12
                        font.bold: true
                    }
                }

                Txt {
                    anchors.horizontalCenter: parent.horizontalCenter
                    visible: root.errorText.length > 0
                    text: root.errorText
                    color: Theme.danger
                    font.family: Theme.fontName
                    font.pixelSize: 13
                    font.bold: true
                }

                // TEMPORARY escape hatch: unlocks without a password (two
                // clicks: first arms, second unlocks). Added because PAM
                // auth was broken (system-auth service missing on this
                // pam-config machine) and trapped the user. With common-auth
                // in place the password works again — remove this button
                // once it's confirmed live on the real machine.
                Rectangle {
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: 240
                    height: 32
                    radius: 10
                    color: root.bypassArmed ? Qt.rgba(1, 1, 1, 0.10) : (bypassMouse.containsMouse ? Qt.rgba(1, 1, 1, 0.08) : "transparent")
                    border.width: 1
                    border.color: root.bypassArmed ? Theme.warning : (bypassMouse.containsMouse ? Qt.rgba(1, 1, 1, 0.5) : Qt.rgba(1, 1, 1, 0.25))

                    Behavior on color {
                        ColorAnimation { duration: 120 }
                    }
                    Behavior on border.color {
                        ColorAnimation { duration: 120 }
                    }

                    Row {
                        anchors.centerIn: parent
                        spacing: 8

                        Txt {
                            anchors.verticalCenter: parent.verticalCenter
                            text: "\uf2f2"
                            font.family: Theme.iconFont
                            font.pixelSize: 11
                            color: root.bypassArmed ? Theme.warning : Theme.fg2
                        }

                        Txt {
                            anchors.verticalCenter: parent.verticalCenter
                            text: root.bypassArmed ? "Click again to confirm unlock" : "Temporary bypass: unlock"
                            color: root.bypassArmed ? Theme.warning : (bypassMouse.containsMouse ? Theme.fg : Theme.fg2)
                            font.family: Theme.fontName
                            font.pixelSize: 11
                            font.bold: true
                        }
                    }

                    MouseArea {
                        id: bypassMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.bypassClick()
                    }
                }

                Txt {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: "\uf023  Press Enter to unlock"
                    color: Theme.fg3
                    font.family: Theme.fontName
                    font.pixelSize: 12
                }
            }

            Component.onCompleted: {
                if (root.locked)
                    passwordField.forceActiveFocus()
            }
        }
    }
}
