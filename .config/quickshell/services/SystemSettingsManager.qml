pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import "."

// Bridge between the settings UI and the hdots runtime state files. Owns
// every path the dotfiles use ($states, $states2, $hypr_bin, $shaders,
// $hypr_global), exposes the active channel/mode, and provides batched
// file reads + single-shot writes + script apply so the schema engine can
// stay pure data. All commands are one-shot bash (no polling, no timers):
// reads happen when a panel opens / a channel changes, writes on user
// action only, so idle cost is zero.
Scope {
    id: root

    readonly property string home: Quickshell.env("HOME") || "/home/tw"
    readonly property string hyprConf: Quickshell.env("rconf") || ("/tmp/" + (Quickshell.env("USER") || "tw") + "_rconf")
    readonly property string states: Quickshell.env("states") || (root.hyprConf + "/states")
    readonly property string states2: Quickshell.env("states2") || (root.hyprConf + "/states2")
    readonly property string hyprBin: Quickshell.env("hypr_bin") || (root.hyprConf + "/hypr_bin")
    readonly property string hyprSources: Quickshell.env("sources") || (root.hyprConf + "/sources")
    readonly property string hyprGlobal: Quickshell.env("hypr_global") || ("/tmp/hypr_global")
    readonly property string hyprGlobalStates: root.hyprGlobal + "/states"
    readonly property string shaders: Quickshell.env("shaders") || (root.home + "/.config/hypr/shaders")
    readonly property string hyprCache: root.hyprConf + "/rcache"
    readonly property string qsScripts: root.hyprConf + "/quickshell/scripts"

    // Active channel ("n" normal | "d" dark | "l" light) and theme mode
    // ("d" | "l"). Read from states2 at startup; changes are surfaced as
    // property changes so open panels re-sync their channel-scoped rows.
    property string channel: "d"
    property string mode: "d"

    // ----- path helpers ---------------------------------------------------

    function pathFor(scope, file) {
        switch (scope) {
        case "channel":
            return root.states + "/" + file + "_" + root.channel
        case "cm":
            // channel+mode scoped (e.g. scheme_<channel><mode>: scheme_dd)
            return root.states + "/" + file + root.channel + root.mode
        case "global":
            return root.states + "/" + file
        case "power":
            return root.hyprGlobalStates + "/" + file
        case "states2":
            return root.states2 + "/" + file
        default:
            return root.states + "/" + file
        }
    }

    function esc(s) {
        return "'" + String(s).replace(/'/g, "'\\''") + "'"
    }

    // ----- environment for scripts ---------------------------------------

    function envExport() {
        const sm = root.channel + root.mode
        return [
            "export hdots=" + root.esc(root.home + "/.config/hdots"),
            "export hdots_sources=" + root.esc(root.home + "/.config/hdots/sources"),
            "export rconf=" + root.esc(root.hyprConf),
            "export hypr_scripts=" + root.esc(root.hyprConf + "/scripts"),
            "export hypr_global=" + root.esc(root.hyprGlobal),
            "export rcache=" + root.esc(root.hyprConf + "/rcache"),
            "export states=" + root.esc(root.states),
            "export states2=" + root.esc(root.states2),
            "export hypr_temp=" + root.esc(root.hyprConf + "/hypr_temp"),
            "export sources=" + root.esc(root.hyprSources),
            "export hypr_bin=" + root.esc(root.hyprBin),
            "export hex_states=" + root.esc(root.hyprConf + "/theme_hex_states"),
            "export theme_elements=" + root.esc(root.hyprConf + "/theme_elements_states"),
            "export shaders=" + root.esc(root.shaders),
            "export sm=" + sm,
            "export qs_scripts=" + root.esc(root.qsScripts),
            "export PATH=" + root.hyprBin + ":" + root.qsScripts + ":" + root.home + "/bin:$PATH",
            "\n"
        ].join("\n")
    }

    // ----- batched reads --------------------------------------------------

    // One bash process prints every requested file (or empty) separated by
    // \x01. cb(array) receives one entry per path, in order. Queued so a
    // burst of readBatch calls collapses into sequential processes.
    property var _readQueue: []
    property var _readCb: null
    property bool _readBusy: false

    function readBatch(paths, cb) {
        root._readQueue.push({ paths: paths || [], cb: cb })
        root._pump()
    }

    function _pump() {
        if (root._readBusy || root._readQueue.length === 0)
            return
        root._readBusy = true
        const task = root._readQueue.shift()
        root._readCb = task.cb
        readWatcher.stop()
        readWatcher.restart()
        const cmd = ["bash", "-c", "for p in \"$@\"; do [[ -f \"$p\" ]] && cat \"$p\" || true; printf '\\x01'; done", "read"]
        for (let i = 0; i < task.paths.length; i++)
            cmd.push(String(task.paths[i]))
        ioProc.command = cmd
        ioProc.running = false
        ioProc.running = true
    }

    // Safety watchdog per queue: if a spawned process never emits
    // onStreamFinished (crashed/blocked bash), the pump flag would stay busy
    // forever and the queue would silently grow, leaking callbacks and
    // freezing the settings UI. Each trip force-clears the current task and
    // requeues the rest so the shell never wedges.
    property Timer readWatcher: Timer {
        interval: 8000
        repeat: false
        onTriggered: {
            root._readCb = null
            root._readBusy = false
            root._pump()
        }
    }
    property Timer linesWatcher: Timer {
        interval: 8000
        repeat: false
        onTriggered: {
            root._readLinesCb = null
            root._linesBusy = false
            root._pumpLines()
        }
    }

    Process {
        id: ioProc
        stdout: StdioCollector {
            // NB: quickshell's StdioCollector has a `text` property and a
            // no-arg streamFinished signal — an arrow param (`text =>`)
            // shadows the property with undefined, so use the implicit
            // scope reference like every other manager in the shell.
            onStreamFinished: {
                root.readWatcher.stop()
                const parts = String(text).split("\x01")
                if (parts.length > 0 && parts[parts.length - 1] === "")
                    parts.pop()
                const cb = root._readCb
                root._readCb = null
                root._readBusy = false
                root._pump()
                if (cb)
                    cb(parts)
            }
        }
    }

    // ----- writes ---------------------------------------------------------

    // Writes a list of { path, value } in one process (no interleaving).
    // value may be a function(display) for dynamic content.
    property var _writeCb: null

    function writeFiles(list, cb) {
        if (!list || list.length === 0) {
            if (cb)
                cb()
            return
        }
        const cmds = []
        for (let i = 0; i < list.length; i++) {
            const w = list[i]
            const v = typeof w.value === "function" ? w.value() : w.value
            // mkdir the target dir defensively (pure bash) so writes work
            // even if a states2/… directory does not exist yet.
            cmds.push("p=" + root.esc(w.path) + "; mkdir -p \"${p%/*}\" && printf '%s\\n' " + root.esc(v) + " > \"$p\"")
        }
        root._writeCb = cb || null
        writeProc.command = ["bash", "-c", cmds.join(" && ") + "; printf done", "write"]
        writeProc.running = false
        writeProc.running = true
    }

    Process {
        id: writeProc
        stdout: StdioCollector {
            onStreamFinished: {
                const cb = root._writeCb
                root._writeCb = null
                if (cb)
                    cb()
            }
        }
    }

    // ----- script apply + raw run ----------------------------------------

    function apply(script, arg, post) {
        let cmd = root.envExport()
        cmd += "exec " + script + (arg ? " " + arg : "")
        if (post)
            cmd += "\n" + post
        cmd += "\nprintf done"
        applyProc.command = ["bash", "-c", cmd, "apply"]
        applyProc.running = false
        applyProc.running = true
    }

    Process {
        id: applyProc
        stdout: StdioCollector { onStreamFinished: root.afterApply() }
    }

    signal applied()

    function afterApply() {
        root.applied()
    }

    function run(cmd) {
        runProc.command = ["bash", "-c", root.envExport() + cmd + "\nprintf done", "run"]
        runProc.running = false
        runProc.running = true
    }

    Process {
        id: runProc
        stdout: StdioCollector { onStreamFinished: root.afterRun() }
    }

    signal ran()

    function afterRun() {
        root.ran()
    }

    // ----- one-shot line list (dynamic combo entries) --------------------

    // readLines shares one Process, so concurrent calls are serialized
    // through a queue (mirrors the readBatch pump): the engine fires two
    // readLines back-to-back when a section loads (saturations + fonts),
    // and without the queue the second call clobbers the first process
    // and its callback never fires.
    property var _linesQueue: []
    property var _readLinesCb: null
    property bool _linesBusy: false

    function readLines(cmd, cb) {
        root._linesQueue.push({ cmd: cmd, cb: cb || null })
        root._pumpLines()
    }

    function _pumpLines() {
        if (root._linesBusy || root._linesQueue.length === 0)
            return
        root._linesBusy = true
        const task = root._linesQueue.shift()
        root._readLinesCb = task.cb
        linesWatcher.stop()
        linesWatcher.restart()
        linesProc.command = ["bash", "-c", task.cmd + "; printf done", "lines"]
        linesProc.running = false
        linesProc.running = true
    }

    Process {
        id: linesProc
        stdout: StdioCollector {
            onStreamFinished: {
                root.linesWatcher.stop()
                const raw = String(text)
                const body = raw.endsWith("done") ? raw.slice(0, -4) : raw
                const cb = root._readLinesCb
                root._readLinesCb = null
                root._linesBusy = false
                root._pumpLines()
                if (cb)
                    cb(body.split("\n"))
            }
        }
    }

    // ----- channel/mode refresh ------------------------------------------

    function refreshChannel() {
        root.readBatch([root.states2 + "/channel"], r => {
            const v = (r[0] || "").trim()
            if (v)
                root.channel = v
        })
        root.readBatch([root.states2 + "/mode"], r => {
            const v = (r[0] || "").trim()
            if (v)
                root.mode = v
        })
    }

    Component.onCompleted: root.refreshChannel()
}
