pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import "."

// Event-driven audio routing state. Sinks/sources are only listed on demand
// (refresh() is called when the audio panel opens / a device is changed), so
// no timer or Process spawns run while idle. The default sink/source is
// re-read after every set, keeping the UI in sync without polling.
Scope {
    id: root

    property var sinks: []
    property var sources: []
    // Running app streams (sink-inputs): { id, app, vol, muted }. Only listed
    // on demand — no polling, so idle cost stays zero.
    property var streams: []
    property bool loading: false

    function refresh() {
        if (root.loading)
            return
        root.loading = true
        root.sinks = []
        root.sources = []
        root.streams = []
        sinksProc.running = true
        sourcesProc.running = true
        streamsProc.running = true
    }

    function setSink(name) {
        setSinkProc.name = name
        setSinkProc.running = true
    }

    function setSource(name) {
        setSourceProc.name = name
        setSourceProc.running = true
    }

    function setStreamVolume(id, pct) {
        setStreamVolProc.id = String(id)
        setStreamVolProc.pct = String(pct)
        setStreamVolProc.running = true
    }

    function setStreamMute(id) {
        setStreamMuteProc.id = String(id)
        setStreamMuteProc.running = true
    }

    Process {
        id: sinksProc
        command: ["audio_main", "audio", "sinks"]
        stdout: SplitParser {
            onRead: line => {
                const parts = String(line).split("\t")
                if (parts.length >= 4) {
                    root.sinks = root.sinks.concat([{
                        id: parts[0],
                        name: parts[1],
                        desc: parts[2],
                        active: parts[3].indexOf("*") !== -1
                    }])
                }
            }
        }
        onExited: root.checkDone()
    }

    Process {
        id: sourcesProc
        command: ["audio_main", "audio", "sources"]
        stdout: SplitParser {
            onRead: line => {
                const parts = String(line).split("\t")
                if (parts.length >= 4) {
                    root.sources = root.sources.concat([{
                        id: parts[0],
                        name: parts[1],
                        desc: parts[2],
                        active: parts[3].indexOf("*") !== -1
                    }])
                }
            }
        }
        onExited: root.checkDone()
    }

    Process {
        id: streamsProc
        command: ["audio_main", "audio", "streams"]
        stdout: SplitParser {
            onRead: line => {
                const parts = String(line).split("\t")
                if (parts.length >= 4) {
                    root.streams = root.streams.concat([{
                        id: parts[0],
                        app: parts[1],
                        vol: parseInt(parts[2]) || 0,
                        muted: parts[3] === "1"
                    }])
                }
            }
        }
        onExited: root.checkDone()
    }

    function checkDone() {
        if (sinksProc.running || sourcesProc.running || streamsProc.running)
            return
        root.loading = false
    }

    Process {
        id: setSinkProc
        property string name: ""
        command: ["audio_main", "audio", "set-sink", setSinkProc.name]
        onExited: root.refresh()
    }

    Process {
        id: setSourceProc
        property string name: ""
        command: ["audio_main", "audio", "set-source", setSourceProc.name]
        onExited: root.refresh()
    }

    Process {
        id: setStreamVolProc
        property string id: ""
        property string pct: ""
        command: ["audio_main", "audio", "stream-vol", setStreamVolProc.id, setStreamVolProc.pct]
        onExited: root.refresh()
    }

    Process {
        id: setStreamMuteProc
        property string id: ""
        command: ["audio_main", "audio", "stream-mute", setStreamMuteProc.id]
        onExited: root.refresh()
    }
}