pragma Singleton

import QtQuick
import Quickshell
import Quickshell.Io
import "."

QtObject {
    id: root

    property string osdMode: "pill"

    // True while the pill is momentarily expanded (always-expanded mode, or
    // hovered open in collapsed-by-default mode). Set by Pill.qml via a
    // property binding; consumed by WeatherManager to gate background polls.
    property bool barExpanded: false

    // Session lock. Flipped by the `bar lock` IPC (power_main `session lock`);
    // drives the full-screen WlSessionLock in LockScreen.qml. Unlock only
    // ever happens through a correct PAM password — no IPC unlock path.
    property bool locked: false

    function lock() {
        osdMode = "pill"
        pillTransitionTimer.stop()
        hideTimer.stop()
        locked = true
    }

    function unlock() {
        locked = false
    }

    function returnToPill() {
        osdMode = "dummy"
        pillTransitionTimer.restart()
    }

    function resetHideTimer() {
        hideTimer.restart()
    }

    function pillReset() {
        returnToPill()
        hideTimer.stop()
    }

    // Power-menu actions run through this long-lived Process (NOT one owned
    // by the PowerPanel): panels are destroyed the moment the action fires
    // (pillReset unloads them), and a destroyed Process takes its onExited /
    // stderr collectors — and possibly the child — with it, so failures
    // would be silent. Owned here, the process, its logging and the failure
    // toast all survive until the command actually finishes.
    property Process powerActionProc: Process {
        stderr: StdioCollector {
            onStreamFinished: console.log("[power] stderr: " + text)
        }
        onExited: function(exitCode) {
            const cmd = powerActionProc.command.length > 2 ? String(powerActionProc.command[2]) : ""
            console.log("[power] action exited " + exitCode + ": " + cmd)
            if (exitCode !== 0)
                root.osdToast("\uf071", "Power action failed", cmd)
        }
    }

    function runPowerAction(command) {
        powerActionProc.command = ["sh", "-c", command]
        console.log("[power] running: " + command)
        powerActionProc.running = true
    }

    function pill() {
        osdMode = "pill"
    }

    function volume() {
        toastTimer.stop()
        osdMode = "volume"
        resetHideTimer()
    }

    function brightness() {
        toastTimer.stop()
        osdMode = "brightness"
        resetHideTimer()
    }

    function micVolume() {
        toastTimer.stop()
        osdMode = "micvolume"
        resetHideTimer()
    }

    // Transient OSD toasts (workspace switch, AC plug/unplug). These morph
    // the bar for a moment and auto-hide, unlike the modal overlays above.
    function osdWorkspace() {
        pillTransitionTimer.stop()
        hideTimer.stop()
        osdMode = "osdworkspace"
        toastTimer.restart()
    }

    function osdPower() {
        pillTransitionTimer.stop()
        hideTimer.stop()
        osdMode = "osdpower"
        toastTimer.restart()
    }

    // Channel switch OSD — fired via IPC by hypr_bin/channel_switcher
    // (bar channel_split / bar channel_normal). No watching, no smart
    // logic: the bash decides the channel and just calls the function.
    property bool channelIsSplit: false
    readonly property string channelIcon: root.channelIsSplit ? "\uf0db" : "\uf108" // fa-columns / fa-desktop
    readonly property string channelTitle: root.channelIsSplit ? "SPLIT" : "NORMAL"
    readonly property string channelSubtitle: root.channelIsSplit
        ? "Switched to Split UI State" : "Switched to Normal UI State"

    function osdChannel(split) {
        pillTransitionTimer.stop()
        hideTimer.stop()
        root.channelIsSplit = !!split
        osdMode = "osdchannel"
        toastTimer.restart()
    }

    // Generic one-shot toast: content lives on this singleton so the caller
    // just picks an icon/title/subtitle. Morphs the bar, auto-hides after
    // toastTimer.
    property string toastIcon: ""
    property string toastTitle: ""
    property string toastSubtitle: ""

    function osdToast(icon, title, subtitle) {
        pillTransitionTimer.stop()
        hideTimer.stop()
        toastIcon = icon
        toastTitle = title
        toastSubtitle = subtitle || ""
        osdMode = "osdgeneric"
        toastTimer.restart()
    }

    // Convenience wrappers for the shell's common toasts.
    function osdMedia() {
        root.osdToast("\uf028", MprisManager.title, MprisManager.artist)
    }

    function osdMic() {
        root.osdToast(VolumeManager.micMuted ? "\uf131" : "\uf130",
            VolumeManager.micMuted ? "Microphone muted" : "Microphone unmuted", "")
    }

    function osdVolumeMute() {
        root.osdToast(VolumeManager.volumeMuted ? "\uf026" : "\uf028",
            VolumeManager.volumeMuted ? "Volume muted" : "Volume unmuted", "")
    }

    function osdScreenshot(name) {
        root.osdToast("\uf03e", "Screenshot saved", name || "Saved to Pictures/Screenshots")
    }

    // Caps Lock OSD toast — fired via IPC by hypr_bin/caps_lock_trigger
    // (bar caps_osd 0|1). The bash owns the LED toggle + 2s hardware
    // re-sync; the shell only renders the drawn lock/lockOpen icon.
    property bool capsLockOn: false

    function capsOsd(on) {
        root.capsLockOn = !!on
        pillTransitionTimer.stop()
        hideTimer.stop()
        osdMode = "oscaps"
        toastTimer.restart()
    }

    function osdRecordStart(name) {
        root.osdToast("\uf111", "Recording started", name || "Recording to Recordings/")
    }

    function osdRecordStop(name) {
        root.osdToast("\uf111", "Recording stopped", name || "Saved to Recordings/")
    }

    function osdClipboard(preview) {
        root.osdToast("\uf0ea", "Copied to clipboard", preview || "")
    }

    // Modes that a toast may safely interrupt: the pill itself, mid-morph
    // placeholders, and other transient OSDs (volume/brightness/toasts).
    function idleForToast() {
        return osdMode === "pill" || osdMode === "dummy" || osdMode === "volume" ||
               osdMode === "micvolume" || osdMode === "brightness" || osdMode === "osdworkspace" ||
               osdMode === "osdpower" || osdMode === "osdgeneric" || osdMode === "osdchannel" ||
               osdMode === "oscaps"
    }

    function isModalMode(m) {
        return !root.idleForToast()
    }


    // Opening a modal panel: re-opening the current one collapses to the
    // pill (toggle behavior). Modal overlays never auto-hide — the user
    // dismisses them with Escape, so any leftover hide timer is cancelled.
    function openModal(name) {
        if (osdMode === name) {
            returnToPill()
        } else {
            osdMode = name
            hideTimer.stop()
        }
    }

    function controlCenter() { openModal("control") }
    function audio() { openModal("audio") }
    function launcher() { openModal("launcher") }
    function wallpaper() { openModal("wallpaper") }
    function workspaceSwitcher() { openModal("workspace") }
    function windowSwitcher() { openModal("window") }
    function mpris() { openModal("mpris") }
    function commandPalette() { openModal("command") }
    function notificationCenter() { openModal("notifications") }
    function powerMenu() { openModal("power") }
    function settings() { openModal("settings") }
    function themes() { openModal("themes") }
    function ai() { openModal("ai") }
    function calendar() { openModal("calendar") }
    function clipboard() { openModal("clipboard") }
    function clipboardImages() { openModal("clipboardimages") }
    function weather() { openModal("weather") }

    // Open Settings on a specific category (e.g. the Keyboard category via
    // the `keyboard_settings` IPC). SettingsPanel reads this on creation
    // and clears it, so a plain `settings()` open always lands on the last
    // used (or first) category.
    property string settingsCategory: ""

    function settingsCategoryOpen(cat) {
        settingsCategory = cat
        // If Settings is already up, don't toggle it closed — the panel
        // watches settingsCategory and re-targets live.
        if (osdMode !== "settings")
            openModal("settings")
    }

    function keyboardSettings() {
        settingsCategoryOpen("keyboard")
    }

    // Driven by PolkitManager: the bar morphs into the auth dialog when a
    // polkit request arrives and collapses back once it resolves.
    function polkit() {
        osdMode = "polkit"
        hideTimer.stop()
    }

    function polkitDone() {
        returnToPill()
    }

    property Timer hideTimer: Timer {
        interval: 3000
        onTriggered: returnToPill()
    }

    property Timer pillTransitionTimer: Timer {
        interval: 100
        onTriggered: osdMode = "pill"
    }

    // Toasts blink and vanish — shorter than the volume/brightness hide timer.
    property Timer toastTimer: Timer {
        interval: 1800
        onTriggered: returnToPill()
    }

    // If a workspace switch happened while a modal overlay was open, defer the
    // confirmation toast until the bar collapses back to the pill.
    property Timer toastAfterPillTimer: Timer {
        interval: 160
        onTriggered: root.osdWorkspace()
    }

    property int toastBaseline: 0
    property bool pendingWorkspaceToast: false

    onOsdModeChanged: {
        if (root.pendingWorkspaceToast && root.isModalMode(osdMode)) {
            root.pendingWorkspaceToast = false
        } else if (osdMode === "pill" && root.pendingWorkspaceToast) {
            root.pendingWorkspaceToast = false
            toastAfterPillTimer.restart()
        }
    }

    // QtObject has no default property, so children must be declared as
    // properties (same pattern as the timers above).
    property Connections hyprlandWatch: Connections {
        target: HyprlandManager
        function onReadyChanged() {
            // Belt-and-suspenders: ensure the OSD baseline is seeded even if
            // the workspace change signal was missed before the watch attached.
            if (HyprlandManager.ready) {
                const ws = HyprlandManager.currentWorkspace
                if (ws > 0)
                    root.toastBaseline = ws
            }
        }

        function onCurrentWorkspaceChanged() {
            const ws = HyprlandManager.currentWorkspace
            if (ws <= 0)
                return
            // First observations only establish the baseline (no startup toast).
            if (!HyprlandManager.ready || root.toastBaseline <= 0) {
                root.toastBaseline = ws
                return
            }
            if (ws === root.toastBaseline)
                return
            root.toastBaseline = ws
            if (root.idleForToast())
                root.osdWorkspace()
            else
                root.pendingWorkspaceToast = true
        }
    }

    property Connections powerWatch: Connections {
        target: PowerManager
        function onAcStateChanged() {
            if (root.idleForToast())
                root.osdPower()
        }
    }

    // Media track-change toast: only while actually playing, and the first
    // observed title just seeds the baseline so the shell doesn't toast the
    // track that was already playing at startup.
    property string mediaBaseline: ""
    property bool mediaSeeded: false

    property Connections mediaWatch: Connections {
        target: MprisManager
        function onTitleChanged() {
            if (!MprisManager.available || MprisManager.status !== "Playing")
                return
            const t = MprisManager.title
            if (!root.mediaSeeded) {
                root.mediaSeeded = true
                root.mediaBaseline = t
                return
            }
            if (t === root.mediaBaseline)
                return
            root.mediaBaseline = t
            if (root.idleForToast() && MprisManager.status === "Playing")
                root.osdMedia()
        }
    }

    // Mic mute/unmute toast. The startup baseline read is skipped via
    // VolumeManager.micInitialized, so only real user toggles toast.
    property Connections micWatch: Connections {
        target: VolumeManager
        function onMicMutedChanged() {
            if (!VolumeManager.micInitialized)
                return
            if (root.idleForToast())
                root.osdMic()
        }
    }

    // Volume mute/unmute toast (PipeWire node signal → volumeMuted). Gated
    // on volumeInitialized so a sink that boots muted doesn't toast.
    property Connections volumeWatch: Connections {
        target: VolumeManager
        function onVolumeMutedChanged() {
            if (!VolumeManager.volumeInitialized)
                return
            if (root.idleForToast())
                root.osdVolumeMute()
        }
    }

    // Reactive volume-level OSD. The CLI (audio_main → wpctl) writes the
    // device and PipeWire's sink node emits onVolumeChanged, which lands in
    // VolumeManager.volumeValue — so the OSD shows the moment a key fires,
    // with no IPC round-trip. A short boot settle window absorbs the
    // startup restore (audio_main re-applies the saved level) and any other
    // first-boot writes so the shell doesn't toast at login. After the
    // window, only real |delta| >= 0.01 changes toast.
    property real volumeOsdBaseline: -1
    property Timer volumeSettleTimer: Timer {
        interval: 2500
        running: true
        onTriggered: root.volumeOsdBaseline = VolumeManager.volumeValue
    }
    property Connections volumeOsdWatch: Connections {
        target: VolumeManager
        function onVolumeValueChanged() {
            if (!VolumeManager.volumeInitialized)
                return
            if (root.volumeSettleTimer.running) {
                root.volumeOsdBaseline = VolumeManager.volumeValue
                root.volumeSettleTimer.restart()
                return
            }
            if (root.volumeOsdBaseline < 0) {
                root.volumeOsdBaseline = VolumeManager.volumeValue
                return
            }
            if (Math.abs(VolumeManager.volumeValue - root.volumeOsdBaseline) < 0.01)
                return
            root.volumeOsdBaseline = VolumeManager.volumeValue
            if (root.idleForToast())
                root.volume()
        }
    }

    // Mic-volume OSD, mirror of the sink watcher above (source node signal).
    property real micOsdBaseline: -1
    property Timer micSettleTimer: Timer {
        interval: 2500
        running: true
        onTriggered: root.micOsdBaseline = VolumeManager.micVolume
    }
    property Connections micOsdWatch: Connections {
        target: VolumeManager
        function onMicVolumeChanged() {
            if (!VolumeManager.micInitialized)
                return
            if (root.micSettleTimer.running) {
                root.micOsdBaseline = VolumeManager.micVolume
                root.micSettleTimer.restart()
                return
            }
            if (root.micOsdBaseline < 0) {
                root.micOsdBaseline = VolumeManager.micVolume
                return
            }
            if (Math.abs(VolumeManager.micVolume - root.micOsdBaseline) < 0.01)
                return
            root.micOsdBaseline = VolumeManager.micVolume
            if (root.idleForToast())
                root.micVolume()
        }
    }

    // Clipboard copy toast (fired by ClipboardManager when the picker or the
    // copied-preview path copies something).
    property Connections clipboardWatch: Connections {
        target: ClipboardManager
        function onCopied() {
            if (root.idleForToast())
                root.osdClipboard(ClipboardManager.lastCopiedPreview)
        }
    }
}