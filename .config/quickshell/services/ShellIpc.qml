import QtQuick
import Quickshell
import Quickshell.Io
import "."

Item {
    // IPC targets are grouped by category. Every handler lives here; external
    // callers (keybinds, scripts, desktop entries) use `quickshell ipc call
    // <category> <function>`. See `qs ipc show` for the full listing.

    IpcHandler {
        target: "panel"

        // Panel / modal launchers
        function control_center() {
            StateController.controlCenter()
        }

        function launcher() {
            StateController.launcher()
        }

        function wallpaper() {
            StateController.wallpaper()
        }

        function workspaces() {
            StateController.workspaceSwitcher()
        }

        function mpris() {
            StateController.mpris()
        }

        function window_switcher() {
            StateController.windowSwitcher()
        }

        function command_palette() {
            StateController.commandPalette()
        }

        function notification_center() {
            StateController.notificationCenter()
        }

        function power_menu() {
            StateController.powerMenu()
        }

        function settings() {
            StateController.settings()
        }

        function ai() {
            StateController.ai()
        }

        // Opens Settings straight on the Keyboard category (keybind-friendly).
        function keyboard_settings() {
            StateController.keyboardSettings()
        }

        // Theme selector grid (dedicated panel + Settings category).
        function themes() {
            StateController.themes()
        }

        function theme_settings() {
            StateController.settingsCategoryOpen("themes")
        }

        function weather() {
            StateController.weather()
        }

        function calendar() {
            StateController.calendar()
        }

        function clipboard() {
            StateController.clipboard()
        }

        function clipboard_images() {
            StateController.clipboardImages()
        }

        function pill() {
            StateController.pill()
        }
    }

    IpcHandler {
        target: "theme"

        // Re-reads fg/bg/acc/sfg from $states2 and applies. Fired by theme_body
        // after every theme apply / mode switch (`quickshell ipc call theme
        // restore`), and by the themes grid after theme_main apply. The shell
        // is a dumb reader: it never computes colors, just re-reads the files.
        function restore() {
            Theme.applyFromStates()
        }
    }

    IpcHandler {
        target: "session"

        // Session / shell control
        function lock() {
            StateController.lock()
        }

        function bar_reset() {
            StateController.pillReset()
        }
    }

    IpcHandler {
        target: "osd"

        // OSD feedback for CLI-driven hardware changes. Volume/mic are fully
        // reactive now — the CLI writes the device (audio_main → wpctl) and the
        // PipeWire node signals drive VolumeManager, whose watches toast the OSD
        // (see StateController.volumeOsdWatch/micOsdWatch/volumeWatch/micWatch),
        // so no IPC is needed for audio. Brightness has no PipeWire source, so
        // the keybind path still tells the shell to render + re-read.
        function osd_change(kind: string) {
            switch (kind) {
            case "brightness":
                StateController.brightness()
                // brightness_main already moved the hardware (keybind path); a
                // write here would use the startup cache and jump the level, so
                // only re-read to keep the slider/CommandPanel honest.
                BrightnessManager.refresh()
                break
            default:
                console.warn("[osd] osd_change: unknown kind '" + kind + "'")
            }
        }

        // External toasts (e.g. shot_main fires this after grim saves).
        // IPC argument types must be explicit (see IpcHandler docs) — untyped
        // QVariant parameters are rejected.
        function screenshot_toast(name: string) {
            StateController.osdScreenshot(name)
        }

        function clipboard_toast(preview: string) {
            StateController.osdClipboard(preview)
        }

        function media_toast() {
            StateController.osdMedia()
        }

        function mic_toast() {
            StateController.osdMic()
        }

        // Channel switch OSD — fired by hypr_bin/channel_switcher
        // (quickshell ipc call osd channel_split|channel_normal). No state
        // watching: the bash decides and just calls this.
        function channel_split() {
            StateController.osdChannel(true)
        }

        function channel_normal() {
            StateController.osdChannel(false)
        }

        // Caps Lock OSD toast, fired by hypr_bin/caps_lock_trigger with the
        // fresh LED state (0/1) as the first argument. No shell-side watching:
        // the bash owns the toggle and the 2s hardware re-sync.
        function caps_osd(on: string) {
            StateController.capsOsd(on === "1")
        }
    }

    IpcHandler {
        target: "capture"

        // Capture IPC — screenshot + screen recording (see CaptureManager).
        // Keybind-friendly: quickshell ipc call capture shot_region
        function shot_full() {
            CaptureManager.shotFull()
        }

        function shot_region() {
            CaptureManager.shotRegion()
        }

        function shot_window() {
            CaptureManager.shotWindow()
        }

        function record_toggle() {
            CaptureManager.recordToggle()
        }

        function record_stop() {
            CaptureManager.recordStop()
        }

        // Opens Settings straight on the Capture category.
        function capture_settings() {
            StateController.settingsCategoryOpen("capture")
        }
    }
}
