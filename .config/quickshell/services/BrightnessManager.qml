pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import "."

Scope {
    id: root

    property real brightnessValue: 0.5
    property real pendingBrightness: 0.5

    Component.onCompleted: {
        brightnessReadProc.running = true
    }

    // value is applied to UI immediately; live=true means "still dragging",
    // so we throttle the actual brightnessctl call instead of firing on every pixel
    function setBrightness(value, live) {
        StateController.resetHideTimer()
        brightnessValue = Math.max(0, Math.min(1, value))
        pendingBrightness = brightnessValue
        if (live === true) {
            writeThrottle.restart()
        } else {
            writeThrottle.stop()
            commitWrite()
        }
    }

    function commitWrite() {
        // Any direct backlight write is the new source of truth: clear a gamma
        // shader that brightness_main left behind (its max-brightness tweak)
        // first, so the shader can't linger on top of the new level.
        root.resetGammaShader()
        brightnessWriteProc.brightness = pendingBrightness
        brightnessWriteProc.running = false
        brightnessWriteProc.running = true
    }

    // Re-read the hardware into brightnessValue. brightness_main's keybind
    // writes go straight to brightnessctl and bypass this file, so without
    // this the cached value (read once at startup) goes stale and a later
    // write would jump the level. ShellIpc calls it after every keybind OSD.
    function refresh() {
        brightnessReadProc.running = false
        brightnessReadProc.running = true
    }

    // Fires brightnessctl at most every 40ms while dragging: instant-feeling UI,
    // but the system call itself is rate-limited so it stays flush and stutter-free.
    Timer {
        id: writeThrottle
        interval: 40
        onTriggered: root.commitWrite()
    }

    Process {
        id: brightnessWriteProc
        property real brightness: 0.5
        command: ["brightnessctl", "set", Math.round(brightness * 100) + "%"]
    }

    // Clear brightness_main's gamma shader only when one is actually active
    // (self-guarded: no-op unless $states/gamma_v > 0), so the slider never
    // forks shader_main on every tick for nothing.
    function resetGammaShader() {
        SystemSettingsManager.readBatch([SystemSettingsManager.states + "/gamma_v"], r => {
            if (!(parseInt((r[0] || "0").trim()) > 0))
                return
            gammaResetProc.running = false
            gammaResetProc.running = true
        })
    }

    Process {
        id: gammaResetProc
        command: ["shader_main", "gamma-reset"]
    }

    Process {
        id: brightnessReadProc
        command: ["brightnessctl", "-m"]
        stdout: SplitParser {
            onRead: line => {
                // "-m" output: "amdgpu_bl1,backlight,4965,8%,65535"
                let match = line.match(/,([0-9]+)%/)
                if (match) root.brightnessValue = parseInt(match[1]) / 100
            }
        }
    }
}