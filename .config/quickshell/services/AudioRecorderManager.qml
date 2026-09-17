pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import "."
Scope {
    id: root
    property bool recording: false
    property string currentFile: ""
    property int elapsed: 0
    property var recordings: []
    property bool confirmDelete: true
    property bool showMicSlider: true
    readonly property string homeDir: Quickshell.env("HOME") || "/home/tw"
    readonly property string recordDir: homeDir + "/Recordings/Audio"

    Timer {
        id: recTimer
        interval: 1000
        repeat: true
        onTriggered: root.elapsed++
    }

    function cfg(key, fallback) {
        const c = SettingsManager.config.audioRecorder || {}
        const v = c[key]
        return v === undefined ? fallback : v
    }

    function setCfg(key, value) {
        SettingsManager.set("audioRecorder", key, value)
    }

    function startRecording(name) {
        if (root.recording)
            return
        let n = String(name || "").trim()
        if (n.length === 0)
            n = "rec_" + timestamp()
        if (!n.endsWith(".wav"))
            n += ".wav"
        const file = root.recordDir + "/" + n
        root.currentFile = file
        root.recording = true
        root.elapsed = 0
        recTimer.restart()
        recordProc.command = [
            "sh", "-c",
            "mkdir -p \"$1\" && pw-record \"$1/$2\"",
            "recorder", root.recordDir, n
        ]
        recordProc.running = true
    }

    function stopRecording() {
        if (!root.recording)
            return
        recordStop.running = true
    }

    function timestamp() {
        const d = new Date()
        const p = n => String(n).padStart(2, "0")
        return "" + d.getFullYear() + p(d.getMonth() + 1) + p(d.getDate()) +
               "-" + p(d.getHours()) + p(d.getMinutes()) + p(d.getSeconds())
    }

    function formatElapsed(s) {
        const m = Math.floor(s / 60)
        const sec = s % 60
        return String(m).padStart(2, "0") + ":" + String(sec).padStart(2, "0")
    }

    function formatSize(bytes) {
        if (bytes < 1024)
            return bytes + " B"
        if (bytes < 1048576)
            return (bytes / 1024).toFixed(1) + " KB"
        return (bytes / 1048576).toFixed(1) + " MB"
    }

    function formatDuration(ms) {
        const sec = Math.floor(ms / 1000)
        const m = Math.floor(sec / 60)
        const s = sec % 60
        return String(m).padStart(2, "0") + ":" + String(s).padStart(2, "0")
    }

    function listRecordings() {
        listProc.running = true
    }

    function play(name) {
        if (playProc.running)
            return
        playProc.command = ["pw-play", root.recordDir + "/" + name]
        playProc.running = true
    }

    function stopPlayback() {
        if (playProc.running)
            playStop.running = true
    }

    function del(name, confirmed) {
        if (root.confirmDelete && !confirmed)
            return
        delProc.command = [
            "sh", "-c", "rm -f \"$1/$2\"",
            "delete", root.recordDir, name
        ]
        delProc.running = true
    }

    Process {
        id: recordProc
        onExited: function(exitCode) {
            root.recording = false
            recTimer.stop()
            if (exitCode === 0) {
                StateController.osdToast("\uf130", "Recording saved", String(root.currentFile).replace(/^.*\//, ""))
                root.listRecordings()
            } else {
                StateController.osdToast("\uf130", "Recording failed", "pw-record exited " + exitCode)
            }
            root.currentFile = ""
        }
    }

    property Process recordStop: Process {
        command: ["sh", "-c", "pkill -INT -x pw-record"]
    }

    Process {
        id: listProc
        command: ["sh", "-c", "mkdir -p \"$1\" && ls -1 \"$1\" 2>/dev/null", "ls", root.recordDir]
        stdout: SplitParser {
            onRead: line => {
                const n = String(line).trim()
                if (n.length > 0 && (n.endsWith(".wav") || n.endsWith(".ogg") || n.endsWith(".flac") || n.endsWith(".mp3") || n.endsWith(".opus")))
                    root.recordings = root.recordings.concat([n])
            }
        }
        onStarted: root.recordings = []
    }

    property Process playProc: Process {}

    property Process playStop: Process {
        command: ["sh", "-c", "pkill -INT -x pw-play"]
    }

    property Process delProc: Process {
        onExited: root.listRecordings()
    }

    Component.onCompleted: {
        root.confirmDelete = root.cfg("confirmDelete", true)
        root.showMicSlider = root.cfg("showMicSlider", true)
        root.listRecordings()
    }
}
