import QtQuick
import Quickshell
import Quickshell.Io
import "."
import "../../components"
import "../../services"

// Theme selector: a wallpaper-style grid of color-scheme cards (bg / fg /
// accent swatches), one card per theme. Dark AND light themes are all listed
// together. Themes are parsed from the master definition file ($states/themes,
// one line per theme) via scripts/theme_main. Clicking a card applies it —
// writes $states/scheme_<cm>, resets the custom-accent flags — and syncs the
// bar's own accent + dark/light mode so everything matches instantly.
// Used both as a standalone panel (bar themes) and inside the Settings app.
Item {
    id: root

    implicitWidth: 720
    implicitHeight: 420

    focus: true

    property var themes: []
    property string current: ""
    property string applying: ""
    property string currentMode: ""

    readonly property string toolPath: SystemSettingsManager.home + "/.config/quickshell/scripts/theme_main"

    function schemeKey() {
        const ch = SystemSettingsManager.channel
        const mode = SystemSettingsManager.mode
        return (ch === "n" ? "n" : mode) + mode
    }

    function refreshCurrent() {
        SystemSettingsManager.readBatch([SystemSettingsManager.states + "/scheme_" + root.schemeKey()], r => {
            root.current = String(r[0] || "").trim()
        })
    }

    function load() {
        listProc.command = [
            "bash", "-c",
            SystemSettingsManager.envExport() + "\nbash " + SystemSettingsManager.esc(root.toolPath) + " list",
            "themes"
        ]
        listProc.running = false
        listProc.running = true
        root.refreshCurrent()
    }

    function applyScheme(id) {
        if (root.applying.length > 0)
            return
        root.applying = id
        applyProc.command = [
            "bash", "-c",
            SystemSettingsManager.envExport() + "\nbash " + SystemSettingsManager.esc(root.toolPath) + " apply " + SystemSettingsManager.esc(id),
            "apply"
        ]
        applyProc.running = true
    }

    function parseList(raw) {
        root.themes = []
        const text = String(raw || "").trim()
        if (text.length === 0)
            return
        try {
            const arr = JSON.parse(text)
            root.themes = arr
        } catch (e) {
            console.log("[themes] parse failed: " + e)
        }
    }

    // default_dark -> "Default Dark" (strip a trailing .conf extension).
    function prettyName(id) {
        const s = String(id || "").replace(/\.conf$/i, "")
        return s.split(/[_.]+/).filter(w => w.length > 0).map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(" ")
    }

    function applyPalette(raw) {
        const text = String(raw || "").trim()
        const lines = text.split("\n").map(l => l.trim()).filter(l => l.startsWith("{"))
        if (lines.length === 0)
            return
        try {
            const p = JSON.parse(lines[lines.length - 1])
            // Instant feedback from the emitted palette; the authoritative
            // pass is theme_main restore below (regenerates $states2 and
            // fires the `theme restore` IPC hook that re-reads the colors).
            const map = { fg:"_fg", fg2:"_fg2", fg3:"_fg3", bg:"_bg", bg2:"_bg2",
                          bg3:"_bg3", hover:"_hover", border:"_border",
                          success:"_success", warning:"_warning",
                          danger:"_danger", info:"_info" }
            for (const k in map) {
                if (p[k])
                    Theme[map[k]] = p[k]
            }
            if (p.acc)
                Theme.acc = p.acc
            if (p.sfg)
                Theme.sfg = p.sfg
            if (p.mode === "d")
                Theme.mode = "dark"
            else if (p.mode === "l")
                Theme.mode = "light"
        } catch (e) {
            console.log("[themes] palette parse failed: " + e)
        }
        // theme_main apply already wrote $states/scheme_<cm> (only if it
        // differs) and fired the right theme_main command itself — a mode flip
        // (dark|light) or acc_changed + restore — so theme_body regenerates
        // $states2 + hyprland/GTK and quickshell picks it up via the IPC hook.
    }

    Process {
        id: listProc
        stdout: StdioCollector {
            onStreamFinished: root.parseList(text)
        }
    }

    Process {
        id: applyProc
        stdout: StdioCollector {
            onStreamFinished: {
                root.applyPalette(text)
                root.applying = ""
            }
        }
        onExited: {
            if (root.applying.length > 0)
                root.applying = ""
            root.refreshCurrent()
        }
    }

    Connections {
        target: SystemSettingsManager
        function onChannelChanged() { root.refreshCurrent() }
        function onModeChanged() { root.refreshCurrent() }
    }

    Component.onCompleted: root.load()
    onVisibleChanged: {
        if (visible)
            root.load()
    }

    Keys.onPressed: event => {
        if (event.key === Qt.Key_Escape) {
            StateController.pillReset()
            event.accepted = true
        }
    }

    Column {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 12

        Row {
            width: parent.width
            height: 28
            spacing: 10

            Txt {
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                text: "Themes"
                color: Theme.fg
                font.family: Theme.fontName
                font.pixelSize: 16
            }

            Txt {
                anchors.verticalCenter: parent.verticalCenter
                text: root.current ? "Active: " + root.current : "Loading…"
                color: Theme.fg3
                font.family: Theme.fontName
                font.pixelSize: 12
            }

            Txt {
                anchors.verticalCenter: parent.verticalCenter
                text: root.applying ? "Applying…" : ""
                color: Theme.acc
                font.family: Theme.fontName
                font.pixelSize: 12
                font.bold: true
            }

            Txt {
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                text: root.themes.length + " themes"
                color: Theme.fg3
                font.family: Theme.fontName
                font.pixelSize: 12
            }
        }

        // Fade the whole grid in (mirrors the wallpaper grid).
        Item {
            width: parent.width
            height: parent.height - 40
            opacity: 0
            scale: 0.98

            Behavior on opacity {
                NumberAnimation { duration: 200; easing.type: Easing.OutCubic }
            }
            Behavior on scale {
                NumberAnimation { duration: 200; easing.type: Easing.OutCubic }
            }

            Timer {
                id: fadeInTimer
                interval: 80
                repeat: false
                running: true
                onTriggered: {
                    parent.opacity = 1
                    parent.scale = 1
                }
            }

            GridView {
                id: grid
                anchors.fill: parent
                cellWidth: 168
                cellHeight: 132
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                cacheBuffer: 400

                // Center the items like the wallpaper grid when they take
                // up less space than the container.
                property int cols: Math.max(1, Math.floor(width / cellWidth))
                property int rows: Math.ceil(count / cols)
                property real contentW: cols * cellWidth
                property real contentH: rows * cellHeight
                x: contentW < width ? (width - contentW) / 2 : 0

                model: root.themes

                delegate: Column {
                    required property int index
                    required property var modelData

                    width: grid.cellWidth
                    height: grid.cellHeight
                    spacing: 4

                    function chip(key) {
                        return modelData.mode === "dual" ? modelData.dark[key] : modelData[key]
                    }

                    function chipBg() {
                        return modelData.mode === "dual" ? modelData.dark.bg : modelData.bg
                    }

                    // Border that reads on both light and dark fills: a dark
                    // translucent border on light swatches, light on dark ones.
                    function swatchBorder(hex) {
                        if (!hex || hex.length < 7)
                            return "#33ffffff"
                        const r = parseInt(hex.substr(1, 2), 16)
                        const g = parseInt(hex.substr(3, 2), 16)
                        const b = parseInt(hex.substr(5, 2), 16)
                        const lum = 0.299 * r + 0.587 * g + 0.114 * b
                        return lum > 150 ? "#33000000" : "#55ffffff"
                    }

                    property bool isDual: modelData.mode === "dual"
                    property bool active: root.current === modelData.id

                    // Card — the theme's own palette as the face. Single-mode
                    // themes (dark or light) fill the card with their own bg /
                    // fg. Dual themes show BOTH halves side by side: dark on
                    // the left, light on the right.
                    Rectangle {
                        anchors.horizontalCenter: parent.horizontalCenter
                        width: 152
                        height: 102
                        radius: 10
                        color: isDual ? "transparent" : chipBg()
                        clip: true

                        // Active ring (accent) vs hover border (fg), like the
                        // wallpaper grid's hover outline. z: 1 so it stays on
                        // top of the opaque card face (single bg OR the two
                        // dual halves), which would otherwise paint over it.
                        Rectangle {
                            z: 1
                            anchors.fill: parent
                            radius: parent.radius
                            color: "transparent"
                            border.width: active ? 2 : (hoverArea.containsMouse ? 2 : 0)
                            border.color: active ? Theme.acc : (hoverArea.containsMouse ? Theme.fg : "transparent")
                        }

                        // ---- Dual face: dark half | light half ----
                        Row {
                            visible: isDual
                            anchors.fill: parent
                            spacing: 0

                            // Dark half (minus the 1px divider so the halves
                            // + divider fit exactly in the card width).
                            Rectangle {
                                width: (parent.width - 1) / 2
                                height: parent.height
                                color: modelData.dark.bg

                                Column {
                                    anchors.fill: parent
                                    anchors.margins: 6
                                    spacing: 3

                                    Row {
                                        width: parent.width
                                        spacing: 4

                                        Txt {
                                            text: "Aa"
                                            color: modelData.dark.fg
                                            font.family: Theme.fontName
                                            font.pixelSize: 14
                                            font.bold: true
                                        }

                                        Txt {
                                            anchors.verticalCenter: parent.verticalCenter
                                            width: parent.width - 22
                                            elide: Text.ElideRight
                                            text: root.prettyName(modelData.id)
                                            color: modelData.dark.fg3
                                            font.family: Theme.fontName
                                            font.pixelSize: 8
                                            font.bold: true
                                        }
                                    }

                                    // bg / accent / fg swatches
                                    Row {
                                        width: parent.width
                                        spacing: 3

                                        Rectangle {
                                            width: 15; height: 10; radius: 2
                                            color: modelData.dark.bg
                                            border.width: 1
                                            border.color: swatchBorder(modelData.dark.bg)
                                        }

                                        Rectangle {
                                            width: 15; height: 10; radius: 2
                                            color: modelData.dark.acc
                                            border.width: 1
                                            border.color: swatchBorder(modelData.dark.acc)
                                        }

                                        Rectangle {
                                            width: 15; height: 10; radius: 2
                                            color: modelData.dark.fg
                                            border.width: 1
                                            border.color: swatchBorder(modelData.dark.fg)
                                        }
                                    }

                                    // Selected-row preview: sfg ON the accent fill.
                                    Rectangle {
                                        width: parent.width
                                        height: 14
                                        radius: 3
                                        color: modelData.dark.acc

                                        Txt {
                                            anchors.centerIn: parent
                                            text: "Aa - selected"
                                            color: modelData.dark.sfg
                                            font.family: Theme.fontName
                                            font.pixelSize: 8
                                            font.bold: true
                                        }
                                    }
                                }
                            }

                            // Divider between the two halves.
                            Rectangle {
                                width: 1
                                height: parent.height
                                color: "#40ffffff"
                            }

                            // Light half
                            Rectangle {
                                width: (parent.width - 1) / 2
                                height: parent.height
                                color: modelData.light.bg

                                Column {
                                    anchors.fill: parent
                                    anchors.margins: 6
                                    spacing: 3

                                    Row {
                                        width: parent.width
                                        spacing: 4

                                        Txt {
                                            text: "Aa"
                                            color: modelData.light.fg
                                            font.family: Theme.fontName
                                            font.pixelSize: 14
                                            font.bold: true
                                        }

                                        Txt {
                                            anchors.verticalCenter: parent.verticalCenter
                                            width: parent.width - 22
                                            elide: Text.ElideRight
                                            text: root.prettyName(modelData.id)
                                            color: modelData.light.fg3
                                            font.family: Theme.fontName
                                            font.pixelSize: 8
                                            font.bold: true
                                        }
                                    }

                                    // bg / accent / fg swatches
                                    Row {
                                        width: parent.width
                                        spacing: 3

                                        Rectangle {
                                            width: 15; height: 10; radius: 2
                                            color: modelData.light.bg
                                            border.width: 1
                                            border.color: swatchBorder(modelData.light.bg)
                                        }

                                        Rectangle {
                                            width: 15; height: 10; radius: 2
                                            color: modelData.light.acc
                                            border.width: 1
                                            border.color: swatchBorder(modelData.light.acc)
                                        }

                                        Rectangle {
                                            width: 15; height: 10; radius: 2
                                            color: modelData.light.fg
                                            border.width: 1
                                            border.color: swatchBorder(modelData.light.fg)
                                        }
                                    }

                                    // Selected-row preview: sfg ON the accent fill.
                                    Rectangle {
                                        width: parent.width
                                        height: 14
                                        radius: 3
                                        color: modelData.light.acc

                                        Txt {
                                            anchors.centerIn: parent
                                            text: "Aa - selected"
                                            color: modelData.light.sfg
                                            font.family: Theme.fontName
                                            font.pixelSize: 8
                                            font.bold: true
                                        }
                                    }
                                }
                            }
                        }

                        // ---- Single face: one palette across the whole card ----
                        Column {
                            visible: !isDual
                            anchors.fill: parent
                            anchors.margins: 10
                            spacing: 4

                            Row {
                                width: parent.width
                                spacing: 6

                                Txt {
                                    text: "Aa"
                                    color: chip("fg")
                                    font.family: Theme.fontName
                                    font.pixelSize: 18
                                    font.bold: true
                                }

                                Txt {
                                    anchors.verticalCenter: parent.verticalCenter
                                    width: parent.width - 40
                                    elide: Text.ElideRight
                                    text: root.prettyName(modelData.id)
                                    color: chip("fg3")
                                    font.family: Theme.fontName
                                    font.pixelSize: 10
                                    font.bold: true
                                }
                            }

                            // bg / accent / fg swatches
                            Row {
                                width: parent.width
                                spacing: 5

                                Rectangle {
                                    width: 22; height: 13; radius: 3
                                    color: chip("bg")
                                    border.width: 1
                                    border.color: swatchBorder(chip("bg"))
                                }

                                Rectangle {
                                    width: 22; height: 13; radius: 3
                                    color: chip("acc")
                                    border.width: 1
                                    border.color: swatchBorder(chip("acc"))
                                }

                                Rectangle {
                                    width: 22; height: 13; radius: 3
                                    color: chip("fg")
                                    border.width: 1
                                    border.color: swatchBorder(chip("fg"))
                                }
                            }

                            // extended palette strip: bg2 bg3 hover border + the
                            // four semantic status colors (success warning danger info)
                            Row {
                                width: parent.width
                                height: 12
                                spacing: 3

                                Repeater {
                                    model: ["bg2", "bg3", "hover", "border"]
                                    Rectangle {
                                        width: (parent.width - 12 - 3 * 4) / 4
                                        height: 12
                                        radius: 3
                                        color: chip(modelData)
                                        border.width: 1
                                        border.color: swatchBorder(chip(modelData))
                                    }
                                }

                                Repeater {
                                    model: ["success", "warning", "danger", "info"]
                                    Rectangle {
                                        width: 12; height: 12; radius: 6
                                        color: chip(modelData)
                                        border.width: 1
                                        border.color: swatchBorder(chip(modelData))
                                    }
                                }
                            }

                            // Selected-row preview: the theme's sfg ON its
                            // accent fill (badge / selection / toggle text).
                            Rectangle {
                                width: parent.width
                                height: 16
                                radius: 4
                                color: chip("acc")

                                Txt {
                                    anchors.centerIn: parent
                                    text: "Aa - selected"
                                    color: chip("sfg")
                                    font.family: Theme.fontName
                                    font.pixelSize: 9
                                    font.bold: true
                                }
                            }
                        }
                    }

                    MouseArea {
                            id: hoverArea
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.applyScheme(modelData.id)
                        }
                    }

                    Txt {
                        anchors.horizontalCenter: parent.horizontalCenter
                        width: 152
                        elide: Text.ElideRight
                        horizontalAlignment: Text.AlignHCenter
                        text: root.prettyName(modelData.id)
                        color: active ? Theme.acc : Theme.fg2
                        font.family: Theme.fontName
                        font.pixelSize: 11
                        font.bold: active
                    }
                }

                ScrollDragger {
                    target: grid
                    thumbWidth: 4
                    minThumbHeight: 30
                }
            }
        }
    }
