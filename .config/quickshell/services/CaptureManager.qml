pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import QsHypr 1.0
import "."

// Screenshot + screen recording for the shell. Driven from IPC
// (`bar shot_full|shot_region|shot_window|record_toggle|record_stop`), the
// Capture settings category, and test buttons. Everything is configurable
// under $states/settings.json "capture":
//   screenshotDir (default ~/Pictures/Screenshots)
//   recordDir     (default ~/Pictures/Recordings)
//   format        (png | jpg)
//   copyScreenshot(bool)
//   fps           (15..120)
//
// All captures run through long-lived Processes owned by this singleton so
// an unloading panel (CaptureCategory / a closed panel) can never kill a
// capture mid-flight. Toasts are fired through StateController.
Scope {
    id: root

    readonly property string homeDir: Quickshell.env("HOME") || "/home/tw"

    // Recording state — could drive a persistent "rec" badge later.
    property bool recording: false
    property string recordingFile: ""

    function cfg(key, fallback) {
        const c = SettingsManager.config.capture || {}
        const v = c[key]
        return v === undefined ? fallback : v
    }

    function shotDir() {
        return root.expandDir(root.cfg("screenshotDir", ""), root.homeDir + "/Pictures/Screenshots")
    }

    function recDir() {
        return root.expandDir(root.cfg("recordDir", ""), root.homeDir + "/Pictures/Recordings")
    }

    function shotFormat() {
        return String(root.cfg("format", "png"))
    }

    function copyShot() {
        return root.cfg("copyScreenshot", false) === true
    }

    function recFps() {
        return Math.max(15, Math.min(120, root.cfg("fps", 60)))
    }

    // Expand a leading ~ and fall back to `fallback` when empty.
    function expandDir(dir, fallback) {
        let d = String(dir || "").trim()
        if (d.length === 0)
            return fallback
        if (d === "~")
            d = root.homeDir
        else if (d.startsWith("~/"))
            d = root.homeDir + d.slice(1)
        return d
    }

    function timestamp() {
        const d = new Date()
        const p = n => String(n).padStart(2, "0")
        return "" + d.getFullYear() + p(d.getMonth() + 1) + p(d.getDate()) +
               "-" + p(d.getHours()) + p(d.getMinutes()) + p(d.getSeconds())
    }

    // Geometry for grim/wf-recorder is "X,Y WxH" (slurp/hyprctl format).
    property string activeWindowGeometry: ""
    function windowGeometry() {
        return String(root.activeWindowGeometry || "").trim()
    }

    // ---- screenshots -----------------------------------------------------

    function shotFull() {
        const file = root.shotDir() + "/shot_" + root.timestamp() + "." + root.shotFormat()
        root.grimProc.command = [
            "sh", "-c",
            "d=$1; f=$2; mkdir -p \"$d\" && grim \"$f\" && [ \"$3\" = 1 ] && wl-copy < \"$f\"",
            "capture", root.shotDir(), file, root.copyShot() ? "1" : "0"
        ]
        root.grimProc.label = file
        root.grimProc.running = true
    }

    function shotRegion() {
        root.pendingCapture = "shot"
        root.regionProc.running = true
    }

    function shotWindow() {
        root.pendingCapture = "window"
        QsHypr.request("activewindow", true, function(text) {
            let geo = ""
            try {
                const data = JSON.parse(String(text || ""))
                const at = data.at || []
                const size = data.size || []
                if (at.length === 2 && size.length === 2)
                    geo = at[0] + "," + at[1] + " " + size[0] + "x" + size[1]
            } catch (e) {}
            root.activeWindowGeometry = geo
            if (root.pendingCapture === "window") {
                root.pendingCapture = ""
                const g = root.windowGeometry()
                if (g.length === 0) {
                    StateController.osdToast("\uf03e", "Screenshot failed", "No active window geometry")
                    return
                }
                const file = root.shotDir() + "/shot_" + root.timestamp() + "." + root.shotFormat()
                root.grimProc.command = [
                    "sh", "-c",
                    "d=$1; f=$2; g=$3; mkdir -p \"$d\" && grim -g \"$g\" \"$f\" && [ \"$4\" = 1 ] && wl-copy < \"$f\"",
                    "capture", root.shotDir(), file, g, root.copyShot() ? "1" : "0"
                ]
                root.grimProc.label = file
                root.grimProc.running = true
            }
        })
    }

    // ---- screen recording ------------------------------------------------

    // Toggle: start (with a slurp region pick) or stop the active recording.
    function recordToggle() {
        if (root.recording) {
            root.recordStop()
            return
        }
        root.pendingCapture = "record"
        root.regionProc.running = true
    }

    function recordStop() {
        stopProc.running = true
    }

    // Called by regionProc when slurp yields a geometry (or cancels).
    function captureWithGeometry(geo) {
        const g = String(geo || "").trim()
        if (g.length === 0) {
            root.pendingCapture = ""
            return
        }
        if (root.pendingCapture === "shot") {
            root.pendingCapture = ""
            const file = root.shotDir() + "/shot_" + root.timestamp() + "." + root.shotFormat()
            root.grimProc.command = [
                "sh", "-c",
                "d=$1; f=$2; g=$3; mkdir -p \"$d\" && grim -g \"$g\" \"$f\" && [ \"$4\" = 1 ] && wl-copy < \"$f\"",
                "capture", root.shotDir(), file, g, root.copyShot() ? "1" : "0"
            ]
            root.grimProc.label = file
            root.grimProc.running = true
        } else if (root.pendingCapture === "record") {
            root.pendingCapture = ""
            root.startRecord(g)
        }
    }

    function startRecord(geo) {
        const file = root.recDir() + "/rec_" + root.timestamp() + ".mkv"
        root.recordingFile = file
        root.recording = true
        StateController.osdRecordStart(file)
        recordProc.command = [
            "sh", "-c",
            "d=$1; f=$2; g=$3; r=$4; mkdir -p \"$d\" && wf-recorder -g \"$g\" -r \"$r\" -f \"$f\"",
            "record", root.recDir(), file, geo, String(root.recFps())
        ]
        recordProc.running = true
    }

    // ---- processes -------------------------------------------------------

    // What a region pick is for: "shot" | "record" | "" ; "window" probes
    // active-window geometry instead of slurp.
    property string pendingCapture: ""

    // Region picker (slurp). Shared by region screenshots and recordings.
    property Process regionProc: Process {
        command: ["slurp"]
        stdout: StdioCollector {
            onStreamFinished: root.captureWithGeometry(text)
        }
    }

    // Screenshot executor. label holds the saved file for the toast.
    property Process grimProc: Process {
        property string label: ""
        stderr: StdioCollector {
            onStreamFinished: {
                const err = String(text || "").trim()
                if (err.length > 0)
                    console.log("[capture] grim: " + err)
            }
        }
        onExited: function(exitCode) {
            if (root.pendingCapture !== "")
                root.pendingCapture = ""
            if (exitCode === 0) {
                StateController.osdScreenshot(String(root.grimProc.label).replace(/^.*\//, ""))
            } else {
                StateController.osdToast("\uf03e", "Screenshot failed", "grim exited " + exitCode)
            }
        }
    }

    // wf-recorder process (single long-lived instance).
    property Process recordProc: Process {
        stderr: StdioCollector {
            onStreamFinished: {
                const err = String(text || "").trim()
                if (err.length > 0)
                    console.log("[capture] wf-recorder: " + err)
            }
        }
        onExited: function(exitCode) {
            const wasRecording = root.recording
            root.recording = false
            if (wasRecording) {
                if (exitCode === 0)
                    StateController.osdRecordStop(String(root.recordingFile).replace(/^.*\//, ""))
                else
                    StateController.osdToast("\uf111", "Recording failed", "wf-recorder exited " + exitCode)
            }
            root.recordingFile = ""
        }
    }

    // SIGINT to wf-recorder so it finalizes the mkv.
    property Process stopProc: Process {
        command: ["sh", "-c", "pkill -INT -x wf-recorder"]
    }
}
