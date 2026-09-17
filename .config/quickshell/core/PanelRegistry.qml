pragma Singleton

import QtQuick
import "."
import "../components"
import "../modules/audio"
import "../modules/calendar"
import "../modules/clipboard"
import "../modules/command"
import "../modules/control"
import "../modules/launcher"
import "../modules/mpris"
import "../modules/notifications"
import "../modules/osd"
import "../modules/polkit"
import "../modules/power"
import "../modules/settings"
import "../modules/wallpaper"
import "../modules/weather"
import "../modules/window"
import "../modules/workspace"
import "../modules/ai"
import "../modules/themes"
import "../services"

// Single wiring point for every panel / OSD the bar can morph into.
// Adding a panel = one line in the qmldir + one entry here + a thin
// openModal() wrapper in StateController. Removing one = delete the same
// three lines. MorphSurface's component map and ShellRoot's keyboard focus
// both derive from this file, so there is nowhere else to keep in sync.
//
// This is also a laziness guarantee: a panel only ever exists while its
// mode is open (MorphContainer's Loader), so closed panels cost 0 RAM and
// the shell stays at ~0% CPU when nothing is happening.
QtObject {
    id: root

    property Component wallpaperPanel: Component { WallpaperPanel {} }
    property Component weatherPanel: Component { WeatherPanel {} }
    property Component controlPanel: Component { ControlPanel {} }
    property Component audioPanel: Component { AudioPanel {} }
    property Component launcherPanel: Component { LauncherPanel {} }
    property Component workspacePanel: Component { WorkspacePanel {} }
    property Component mprisPanel: Component { MprisPanel {} }
    property Component windowPanel: Component { WindowPanel {} }
    property Component commandPanel: Component { CommandPanel {} }
    property Component notificationPanel: Component { NotificationPanel {} }
    property Component powerPanel: Component { PowerPanel {} }
    property Component settingsPanel: Component { SettingsPanel {} }
    property Component calendarPanel: Component { CalendarPanel {} }
    property Component clipboardPanel: Component { ClipboardPanel {} }
    property Component clipboardImagesPanel: Component { ClipboardPanel { imageMode: true } }
    property Component polkitPanel: Component { PolkitPanel {} }
    property Component aiPanel: Component { AIPanel {} }
    property Component themesPanel: Component { ThemesPanel {} }

    property Component volumeSlider: Component {
        Slider {
            sliderValue: VolumeManager.volumeValue
            onSetValue: (value, live) => VolumeManager.setVolume(value, live)
            iconType: "volume"
        }
    }

    property Component brightnessSlider: Component {
        Slider {
            sliderValue: BrightnessManager.brightnessValue
            onSetValue: (value, live) => BrightnessManager.setBrightness(value, live)
            iconType: "brightness"
        }
    }

    property Component micVolumeSlider: Component {
        Slider {
            sliderValue: VolumeManager.micVolume
            onSetValue: (value, live) => VolumeManager.setMicVolume(value, live)
            iconType: "mic"
        }
    }

    property Component osdWorkspaceToast: Component {
        OsdToast {
            icon: "\uf108"
            iconType: "monitor"
            title: {
                const id = HyprlandManager.currentWorkspace
                const name = String(HyprlandManager.workspaceNames[String(id)] || "").trim()
                return name.length > 0 && name !== String(id) ? name : "Workspace " + id
            }
        }
    }

    property Component osdPowerToast: Component {
        OsdToast {
            icon: PowerManager.powerIcon
            // Drawn: bolt while plugged in, battery shape with the real level
            // on battery (Apple-style).
            iconType: PowerManager.acOnline ? "bolt" : "battery"
            iconLevel: PowerManager.batteryPercent / 100
            iconCharging: PowerManager.charging
            title: PowerManager.powerTitle
            subtitle: PowerManager.powerSubtitle
        }
    }

    // Generic transient toast driven by StateController (media change, mic
    // mute/unmute, screenshot saved, clipboard copied). Content lives in
    // StateController so any component can fire one with a one-liner.
    property Component osdGenericToast: Component {
        OsdToast {
            icon: StateController.toastIcon
            title: StateController.toastTitle
            subtitle: StateController.toastSubtitle
        }
    }

    // Caps Lock OSD — driven via IPC by hypr_bin/caps_lock_trigger
    // (bar caps_osd 0|1). Drawn lock/lockOpen icon, content on StateController.
    property Component osdCapsToast: Component {
        OsdToast {
            iconType: StateController.capsLockOn ? "lock" : "lockOpen"
            title: StateController.capsLockOn ? "Caps Lock On" : "Caps Lock Off"
        }
    }

    // Channel switch toast — fired via IPC by hypr_bin/channel_switcher
    // (bar channel_split / bar channel_normal). Content lives on
    // StateController, same as the generic toast.
    property Component osdChannelToast: Component {
        OsdToast {
            icon: StateController.channelIcon
            title: StateController.channelTitle
            subtitle: StateController.channelSubtitle
        }
    }

    // mode -> component the island morphs into. MorphSurface merges this
    // with its local "dummy" morph placeholder.
    property var componentMap: ({
        "volume": root.volumeSlider,
        "brightness": root.brightnessSlider,
        "micvolume": root.micVolumeSlider,
        "wallpaper": root.wallpaperPanel,
        "weather": root.weatherPanel,
        "control": root.controlPanel,
        "audio": root.audioPanel,
        "launcher": root.launcherPanel,
        "workspace": root.workspacePanel,
        "mpris": root.mprisPanel,
        "window": root.windowPanel,
        "command": root.commandPanel,
        "notifications": root.notificationPanel,
        "power": root.powerPanel,
        "settings": root.settingsPanel,
        "calendar": root.calendarPanel,
        "clipboard": root.clipboardPanel,
        "clipboardimages": root.clipboardImagesPanel,
        "ai": root.aiPanel,
        "themes": root.themesPanel,
        "polkit": root.polkitPanel,
        "osdworkspace": root.osdWorkspaceToast,
        "osdpower": root.osdPowerToast,
        "osdgeneric": root.osdGenericToast,
        "osdchannel": root.osdChannelToast,
        "oscaps": root.osdCapsToast
    })

    // Modal overlays that must grab exclusive keyboard focus while open
    // (drives ShellRoot's WlrLayershell.keyboardFocus). Transient sliders
    // and OSD toasts auto-hide and never take focus.
    property var exclusiveModes: [
        "wallpaper", "weather", "launcher", "control", "audio", "workspace", "mpris", "window",
        "command", "notifications", "power", "settings", "calendar",
        "clipboard", "clipboardimages", "ai", "themes", "polkit"
    ]
}