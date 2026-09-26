pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import "."
import "../components"

// Recurring special occasions ("MM-DD" -> name), loaded from
// ~/.config/hdots/calendar_events.json and watched so hand-edits apply live.
// Example file:
//   { "events": { "08-07": "My birthday", "12-25": "Christmas" } }
// A day may map to a string or an array of strings.
Scope {
    id: root

    readonly property string eventsPath: (Quickshell.env("HOME") || "/home/tw") + "/.config/hdots/calendar_events.json"
    property var events: ({})
    property bool ready: false

    signal occasionsChanged()

    function pad2(n) {
        return n < 10 ? "0" + n : String(n)
    }

    function monthKey(month, day) {
        return root.pad2(month + 1) + "-" + root.pad2(day)
    }

    // Names (always an array) for one day. `month` is 0-11.
    function eventsOn(month, day) {
        const hit = root.events[root.monthKey(month, day)]
        if (hit === undefined || hit === null)
            return []
        return Array.isArray(hit) ? hit : [hit]
    }

    // Sorted [{ day, names: [] }] for a whole month (side panel of the
    // calendar). `year` is used only for leap-year February day counts.
    function eventsForMonth(year, month) {
        const out = []
        const days = new Date(year, month + 1, 0).getDate()
        for (let d = 1; d <= days; d++) {
            const names = root.eventsOn(month, d)
            if (names.length > 0)
                out.push({ day: d, names: names })
        }
        return out
    }

    function parse(text) {
        let fileCfg = {}
        try {
            fileCfg = JSON.parse(String(text || "") || "{}")
        } catch (e) {
            fileCfg = {}
        }
        root.events = fileCfg.events || {}
        root.ready = true
        root.occasionsChanged()
    }

    // FileView is used only as a change signal (same caveat as
    // SettingsManager: its text() can be stale/empty). Always read fresh.
    function reload() {
        loadRunner.command = ["bash", "-c", "cat \"$1\" 2>/dev/null", "cfg", root.eventsPath]
        loadRunner.run()
    }

    property CommandRunner loadRunner: CommandRunner {
        onFinished: root.parse(loadRunner.text)
    }

    FileView {
        id: changeWatcher
        path: root.eventsPath
        watchChanges: true
        blockLoading: false
        onFileChanged: root.reload()
    }

    Component.onCompleted: {
        root.ready = true
        root.reload()
    }
}