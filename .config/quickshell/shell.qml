//@ pragma UseQApplication
//@ pragma Env QT_QPA_PLATFORMTHEME=gtk3
//@ pragma Env QS_NO_RELOAD_POPUP=1
//@ pragma Env QSG_RENDER_LOOP=threaded
//@ pragma Env QT_QUICK_FLICKABLE_WHEEL_DECELERATION=10000

import Quickshell
import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Shapes
import "components"
import "services"
import "core"
import "modules/notifications"

// this is shell.qml
Scope {
    id: root

    property bool showBar: true
    // Touch the settings singleton at startup so the persisted theme,
    // notification timeout and hide delay apply before anything opens.
    property bool settingsReady: SettingsManager.ready

    // Touch the polkit singleton so its agent registers at startup (QML
    // singletons are created lazily on first reference). It drives the bar
    // morph whenever an auth request arrives.
    property bool polkitEnabled: PolkitManager.isActive

    // Touch singletons so they start polling at shell startup (they cannot be
    // instantiated with `{}`).
    property bool powerReady: PowerManager.ready
    property bool hyprReady: HyprlandManager.ready
    property bool calendarReady: CalendarManager.ready

    // Touch the capture singleton so its Processes (slurp, grim,
    // wf-recorder) stay alive across panel unloads.
    property bool captureReady: CaptureManager.recording === false

    ShellRoot {
        barVisible: root.showBar
    }

    ToastDaemon {}

    // Session lock (ext-session-lock-v1). `StateController.lock()` via the
    // `bar lock` IPC / power_main `session lock` flips its `locked` binding.
    LockScreen {}
}
