import QtQuick
import QtQml
import Quickshell
import "."

// Schema engine for the dotfiles settings UI. Turns a JSON schema section
// into an array of row objects that SettingItem delegates render. Reads
// current values through SystemSettingsManager.readBatch() (one bash call
// per refresh) and commits changes through writeFiles() + the apply script
// of each row. Rows are stable QObjects, so delegate bindings update live
// when values change and focus is preserved across refreshes.
QtObject {
    id: root

    property string schemaFile: ""
    property var sections: []          // one or more schema section names
    property string defaultScope: "channel"
    property var rows: []              // row QObjects (see rowObjectCmp)
    property bool ready: false
    property string error: ""

    signal loaded()
    signal writeDone(string key)

    property var _schema: null
    property bool _loading: false

    // Note: declared as a property (not a child object) — quickshell's
    // QML compiler rejects a Component declared directly under a QtObject
    // root ("Cannot assign to non-existent default property").
    property Component rowObjectCmp: Component {
        QtObject {
            id: r
            property string type: ""
            property string key: ""
            property string label: ""
            property string hint: ""
            property string value: ""
            property string raw: ""
            property string display: ""
            property real min: 0
            property real max: 1
            property real step: 1
            property int currentIndex: -1
            property int listHeight: 0
            property bool enabled: true
            property string kind: ""
            property string entriesDynamic: ""
            property string writePath: ""
            property var entries: []
            property var entryValues: []
            property var readPaths: []
            property var extraWrites: []
            property string applyScript: ""
            property string applyArg: ""
        }
    }

    function _newRow() {
        return rowObjectCmp.createObject(root)
    }

    function findRow(pred) {
        for (let i = 0; i < root.rows.length; i++)
            if (pred(root.rows[i]))
                return root.rows[i]
        return null
    }

    function findAllRows(pred) {
        const out = []
        for (let i = 0; i < root.rows.length; i++)
            if (pred(root.rows[i]))
                out.push(root.rows[i])
        return out
    }

    function resolve(s) {
        if (s.indexOf("{") < 0)
            return s
        const modeWord = SystemSettingsManager.mode === "l" ? "light" : "dark"
        return String(s)
            .replace(/\{channel\}/g, SystemSettingsManager.channel)
            .replace(/\{mode\}/g, modeWord)
    }

    // ----- build -----------------------------------------------------------

    function load() {
        if (root._loading)
            return
        root._loading = true
        if (!root.schemaFile)
            root.schemaFile = (Quickshell.env("PWD") || "/home/tw/.config/quickshell") + "/services/settings-schema.json"
        SystemSettingsManager.readBatch([root.schemaFile], r => {
            try {
                root._schema = JSON.parse(r[0] || "{}")
            } catch (e) {
                root._schema = {}
                root.error = String(e)
            }
            root._build()
            root._loading = false
        })
    }

    function _build() {
        const secs = Array.isArray(root.sections) ? root.sections : [root.sections]
        const all = []
        for (const name of secs) {
            const sec = (root._schema.sections || []).find(s => s.name === name)
            if (!sec)
                continue
            const sectionScope = sec.scope || root.defaultScope
            const sectionApply = sec.apply
            for (const sr of (sec.rows || [])) {
                const row = root._newRow()
                row.type = sr.type
                row.key = sr.key || ""
                row.label = sr.label || ""
                row.hint = sr.hint || ""
                row.kind = sr.kind || ""
                row.entriesDynamic = sr.entriesDynamic || ""
                row.min = sr.min || 0
                row.max = (sr.max !== undefined) ? sr.max : 1
                row.step = sr.step || 1
                row.enabled = sr.enabled !== false

                const scope = sr.scope || sectionScope
                const file = sr.file || (sr.type === "header" || sr.type === "button" ? "" : sr.key) || ""
                row.writePath = file ? SystemSettingsManager.pathFor(scope, file) : ""

                if (sr.entries) {
                    const ents = []
                    const vals = []
                    for (const e of sr.entries) {
                        if (e && typeof e === "object") {
                            ents.push(e.label)
                            vals.push(String(e.value))
                        } else {
                            ents.push(e)
                            vals.push(String(e))
                        }
                    }
                    row.entries = ents
                    row.entryValues = vals
                }

                if (row.kind === "scheme") {
                    row.readPaths = [{ path: row.writePath, field: "main" }]
                } else if (row.kind === "saturation") {
                    row.readPaths = [
                        { path: SystemSettingsManager.pathFor("channel", "shader_state"), field: "state" },
                        { path: SystemSettingsManager.pathFor("channel", "shader_val"), field: "val" }
                    ]
                } else if (row.writePath) {
                    row.readPaths = [{ path: row.writePath, field: "main" }]
                }

                const ap = sr.apply || sectionApply
                if (ap) {
                    row.applyScript = ap.script || ""
                    row.applyArg = root.resolve(ap.arg || "")
                    // theme_main restore implies "accent changed, reapply" so the
                    // watchers pick the new accent (mirrors theme_menu).
                    if (row.applyScript === "theme_main" && row.applyArg.indexOf("restore") >= 0 && row.kind !== "scheme" && row.kind !== "defaults")
                        row.extraWrites = row.extraWrites.concat([{ scope: "states2", file: "acc_changed", value: "1" }])
                }

                if (row.kind === "scheme" || row.kind === "defaults") {
                    row.extraWrites = row.extraWrites.concat([
                        { scope: "channel", file: "custom_acc", value: "0" },
                        { scope: "channel", file: "acc_from_wall", value: "0" },
                        { scope: "channel", file: "acc_from_hex", value: "0" },
                        { scope: "states2", file: "acc_changed", value: "1" }
                    ])
                }

                all.push(row)
            }
        }
        root.rows = all
        root._loadDynamicEntries(() => root.refresh())
    }

    // ----- dynamic combo entries ------------------------------------------

    function _loadDynamicEntries(cb) {
        let pending = 0
        let finished = () => {
            if (--pending <= 0)
                cb()
        }

        // Populate every matching row — several dynamic kinds now appear on
        // more than one row (fonts: font + rofi_font; freqcache: max_ac +
        // max_bat), so assign to all matches, not just findRow's first.
        const schemeRows = root.findAllRows(r => r.kind === "scheme")
        if (schemeRows.length > 0) {
            pending++
            SystemSettingsManager.readBatch([SystemSettingsManager.hyprCache + "/scheme_cache"], r => {
                const seen = {}
                const names = []
                for (const line of String(r[0] || "").split("\n")) {
                    const name = line.substring(line.lastIndexOf("\t") + 1).trim()
                    if (!name || name === ".." || name.indexOf("Refresh") === 0 || name.indexOf("Default") === 0 || seen[name])
                        continue
                    seen[name] = 1
                    names.push(name)
                }
                names.sort()
                for (const row of schemeRows) {
                    row.entries = names
                    row.entryValues = names
                }
                finished()
            })
        }

        const satRows = root.findAllRows(r => r.kind === "saturation")
        if (satRows.length > 0) {
            pending++
            SystemSettingsManager.readLines("printf '%s\\n' " + SystemSettingsManager.esc(SystemSettingsManager.shaders + "/saturations/*.glsl"), lines => {
                const files = []
                for (const l of lines) {
                    const t = String(l).trim()
                    if (t)
                        files.push(t)
                }
                files.sort((a, b) => parseInt(a.split("/").pop()) - parseInt(b.split("/").pop()))
                const ents = ["System Default (shader off)"]
                const vals = ["off"]
                for (const f of files) {
                    const n = parseInt(f.split("/").pop())
                    vals.push(f)
                    ents.push(isNaN(n) ? f.split("/").pop() : (n * 10) + "%")
                }
                for (const row of satRows) {
                    row.entries = ents
                    row.entryValues = vals
                }
                finished()
            })
        }

        const fontRows = root.findAllRows(r => r.entriesDynamic === "fonts")
        if (fontRows.length > 0) {
            pending++
            SystemSettingsManager.readLines("fc-list : family | sort -u", lines => {
                const seen = {}
                const names = []
                for (const l of lines) {
                    const fam = String(l).split(",")[0].split(":")[0].trim()
                    if (!fam || seen[fam])
                        continue
                    seen[fam] = 1
                    names.push(fam)
                }
                names.sort()
                for (const row of fontRows) {
                    row.entries = names
                    row.entryValues = names
                }
                finished()
            })
        }

        // Custom accent list: markup rows from the launcher cache
        // (<span color="#hex">●</span> name (#hex)) — keep name + hex.
        const accRows = root.findAllRows(r => r.entriesDynamic === "accents")
        if (accRows.length > 0) {
            pending++
            SystemSettingsManager.readLines("cat " + SystemSettingsManager.esc(SystemSettingsManager.hyprCache + "/launcher_cache/temp_for_" + SystemSettingsManager.mode), lines => {
                const ents = []
                const vals = []
                for (const l of lines) {
                    const t = String(l).trim()
                    if (!t)
                        continue
                    const display = t.substring(t.lastIndexOf(">") + 1).trim()
                    const name = display.split(" (")[0].trim()
                    if (!name || name === "..")
                        continue
                    vals.push(name)
                    ents.push(display)
                }
                for (const row of accRows) {
                    row.entries = ents
                    row.entryValues = vals
                }
                finished()
            })
        }

        // Favourite accents list ($states/fav_accents_list).
        const favRows = root.findAllRows(r => r.entriesDynamic === "favorites")
        if (favRows.length > 0) {
            pending++
            SystemSettingsManager.readLines("cat " + SystemSettingsManager.esc(SystemSettingsManager.states + "/fav_accents_list"), lines => {
                const ents = []
                const vals = []
                for (const l of lines) {
                    const t = String(l).trim()
                    if (t && t !== "..") {
                        ents.push(t)
                        vals.push(t)
                    }
                }
                for (const row of favRows) {
                    row.entries = ents
                    row.entryValues = vals
                }
                finished()
            })
        }

        // Saved hex colors (~/Documents/saved_random_hex).
        const hexRows = root.findAllRows(r => r.entriesDynamic === "hexlist")
        if (hexRows.length > 0) {
            pending++
            SystemSettingsManager.readLines("cat " + SystemSettingsManager.esc(SystemSettingsManager.home + "/Documents/saved_random_hex"), lines => {
                const ents = []
                const vals = []
                for (const l of lines) {
                    const t = String(l).trim()
                    if (/^#[0-9a-fA-F]{6}$/.test(t)) {
                        ents.push(t)
                        vals.push(t)
                    }
                }
                for (const row of hexRows) {
                    row.entries = ents
                    row.entryValues = vals
                }
                finished()
            })
        }

        // CPU frequency steps cached by power_tune ($hypr_global/cpu_freq_cache).
        const freqRows = root.findAllRows(r => r.entriesDynamic === "freqcache")
        if (freqRows.length > 0) {
            pending++
            SystemSettingsManager.readLines("cat " + SystemSettingsManager.esc(SystemSettingsManager.hyprGlobal + "/cpu_freq_cache"), lines => {
                const ents = []
                const vals = []
                for (const l of lines) {
                    const t = String(l).trim()
                    if (/^[0-9]+$/.test(t)) {
                        ents.push(t)
                        vals.push(t)
                    }
                }
                for (const row of freqRows) {
                    row.entries = ents
                    row.entryValues = vals
                }
                finished()
            })
        }

        if (pending === 0)
            cb()
    }

    // ----- read / refresh --------------------------------------------------

    function refresh() {
        const tasks = []
        for (const row of root.rows) {
            if (row.readPaths.length === 0)
                continue
            for (const rp of row.readPaths)
                tasks.push(rp.path)
        }
        if (tasks.length === 0) {
            root.ready = true
            root.loaded()
            return
        }
        SystemSettingsManager.readBatch(tasks, vals => {
            let i = 0
            for (const row of root.rows) {
                if (row.readPaths.length === 0)
                    continue
                const c = {}
                for (const rp of row.readPaths)
                    c[rp.field] = vals[i++] || ""
                root._applyRaw(row, c)
            }
            root._postProcess()
            root.ready = true
            root.loaded()
        })
    }

    function _applyRaw(row, c) {
        if (row.type === "header" || row.type === "button")
            return
        if (row.type === "toggle") {
            const t = c.main.trim()
            row.raw = t
            row.value = t === "1" ? "1" : "0"
            return
        }
        if (row.type === "slider") {
            if (row.kind === "alpha") {
                // Alpha rows store a 2-hex-digit value (e.g. "bf") exactly
                // like the rofi alpha submenus; the slider works in %.
                const v = root._clamp(root._hexToPct(c.main), row)
                row.raw = c.main.trim()
                row.value = root._formatSlider(v, row)
                row.display = row.value + "%"
                return
            }
            const v = root._clamp(parseFloat(c.main), row)
            row.raw = c.main.trim()
            row.value = root._formatSlider(v, row)
            row.display = row.value
            return
        }
        // combo
        if (row.kind === "saturation") {
            const state = c.state.trim()
            const val = c.val.trim()
            row.raw = state
            if (state !== "1") {
                row.currentIndex = 0
                row.display = row.entries.length ? row.entries[0] : ""
            } else {
                const idx = row.entryValues.indexOf(val)
                row.currentIndex = idx >= 0 ? idx : 0
                row.display = idx >= 0 ? row.entries[idx] : ""
            }
            return
        }
        const raw = c.main.trim()
        row.raw = raw
        const idx = row.entryValues.indexOf(raw)
        row.currentIndex = idx >= 0 ? idx : -1
        row.display = idx >= 0 ? row.entries[idx] : (raw || "")
    }

    function _postProcess() {
        const st = root.findRow(r => r.kind === "sunstate")
        const val = root.findRow(r => r.kind === "sunval")
        if (st && val)
            val.enabled = st.raw === "m"
    }

    function _clamp(n, row) {
        if (isNaN(n))
            n = row.min
        return Math.max(row.min, Math.min(row.max, n))
    }

    function _formatSlider(v, row) {
        let n = root._clamp(Number(v), row)
        n = Math.round((n - row.min) / row.step) * row.step + row.min
        n = Math.round(n * 1000) / 1000
        return row.step < 1 ? n.toFixed(1) : String(Math.round(n))
    }

    // Alpha rows: state stores 2-hex-digit alpha, the UI works in %.
    // pctToHex floors like bash's integer division in the rofi submenus
    // (p * 255 / 100); hexToPct rounds so a written value reads back as
    // the same slider position.
    function _pctToHex(p) {
        const n = Math.max(0, Math.min(100, Number(p) || 0))
        const h = Math.floor(n * 255 / 100).toString(16)
        return h.length < 2 ? "0" + h : h
    }

    function _hexToPct(h) {
        const n = parseInt(String(h).trim(), 16)
        if (isNaN(n))
            return 0
        return Math.round(n * 100 / 255)
    }

    // ----- write -----------------------------------------------------------

    function setValue(key, value) {
        const row = root.findRow(r => r.key === key)
        if (!row)
            return

        if (row.type === "button") {
            root._commitWrites(row, [])
            return
        }

        let writeValue = ""
        if (row.type === "toggle")
            writeValue = value ? "1" : "0"
        else if (row.type === "slider") {
            writeValue = root._formatSlider(value, row)
            if (row.kind === "alpha")
                writeValue = root._pctToHex(parseFloat(writeValue))
        }
        else if (row.type === "combo") {
            if (typeof value === "number")
                writeValue = row.entryValues[value] !== undefined ? row.entryValues[value] : "off"
            else
                writeValue = value
        }
        root._applyWrite(row, writeValue)

        const writes = row.writePath ? [{ path: row.writePath, value: writeValue }] : []
        for (const e of row.extraWrites)
            writes.push({ path: SystemSettingsManager.pathFor(e.scope, root.resolve(e.file)), value: root.resolve(e.value) })
        root._commitWrites(row, writes)
    }

    function _applyWrite(row, v) {
        if (row.type === "toggle") {
            row.raw = v
            row.value = v
            return
        }
        if (row.type === "slider") {
            if (row.kind === "alpha") {
                row.raw = v
                const pct = root._hexToPct(v)
                row.value = root._formatSlider(pct, row)
                row.display = row.value + "%"
                return
            }
            row.raw = v
            row.value = v
            row.display = v
            return
        }
        // combo
        if (row.kind === "saturation") {
            if (v === "off") {
                row.raw = "0"
                row.currentIndex = 0
                row.display = row.entries.length ? row.entries[0] : ""
            } else {
                row.raw = "1"
                const idx = row.entryValues.indexOf(v)
                row.currentIndex = idx >= 0 ? idx : 1
                row.display = idx >= 0 ? row.entries[idx] : ""
            }
            return
        }
        const idx = row.entryValues.indexOf(v)
        row.currentIndex = idx >= 0 ? idx : -1
        row.display = idx >= 0 ? row.entries[idx] : (v || "")
        row.raw = v
        if (row.kind === "sunstate")
            root._postProcess()
    }

    function _commitWrites(row, writes) {
        let applyScript = row.applyScript
        let applyArg = row.applyArg

        // Scripts can receive the value just written via a {value} placeholder
        // (e.g. alpha_main acc_alpha {value}) — substituted from row.raw here,
        // after the write so the script sees the committed state.
        if (applyArg && applyArg.indexOf("{value}") >= 0)
            applyArg = applyArg.replace(/\{value\}/g, row.raw)

        if (row.kind === "saturation") {
            const on = row.raw === "1"
            writes.push({ path: SystemSettingsManager.pathFor("channel", "shader_state"), value: on ? "1" : "0" })
            applyScript = "shader_main"
            applyArg = on ? "on" : "off"
        }
        if (row.kind === "sunstate") {
            const st = row.raw
            applyScript = "hyprsunset_main"
            applyArg = st === "c" ? "off" : st === "m" ? "manual" : "on"
        }
        if (row.kind === "sunval") {
            applyScript = "hyprsunset_main"
            applyArg = "manual"
        }

        SystemSettingsManager.writeFiles(writes, () => {
            if (applyScript)
                SystemSettingsManager.apply(applyScript, applyArg)
            root.writeDone(row.key)
        })
    }
}
