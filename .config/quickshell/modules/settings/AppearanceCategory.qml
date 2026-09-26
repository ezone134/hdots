import QtQuick
import QtQml
import "../../components"
import "../../services"

// Appearance category: accent color source (4 options + Apply that only
// writes the $states flags), font family and icon theme.
//
// Accent section is a pure delegator. Choosing an option + Apply runs
// `theme_main accent <key> [hex]` — theme_main owns ALL flag writes (it reads
// the active channel itself). Keys: theme_default | acc_from_wall |
// define_hex <hex> | custom_acc <name>. Apply is IDEMPOTENT: if the
// chosen option matches the currently applied source, it does nothing —
// no theme_main call at all.
Flickable {
    id: root
    clip: true
    contentHeight: col.height
    boundsBehavior: Flickable.StopAtBounds

    // ----- accent source state ------------------------------------------

    readonly property string ch: SystemSettingsManager.channel

    // Options the UI offers.
    property string selSource: "theme"        // theme | wall | hex | custom
    property string selCustomName: ""
    property string selHex: ""

    // Currently applied source (read from $states on load / after apply).
    property string activeSource: "theme"
    property string activeCustomName: ""
    property string activeHex: ""

    // Custom accent list from the launcher cache ([{name,d,l}]).
    property var accents: []

    // Accent tint toggles (0/1 flags theme_body picks up).
    property bool hyprlandToggle: false

    // Hex to apply for the "defined hex" option (picked or from Documents).
    readonly property string resolvedHex: {
        if (selHex)
            return selHex
        if (activeSource === "hex" && activeHex)
            return activeHex
        return ""
    }

    // Whether the pending selection differs from the applied one.
    readonly property bool isDirty: {
        if (selSource !== activeSource)
            return true
        if (selSource === "custom" && selCustomName && selCustomName !== activeCustomName)
            return true
        if (selSource === "hex" && resolvedHex && resolvedHex !== activeHex)
            return true
        return false
    }

    function flagPath(flag) {
        return SystemSettingsManager.states + "/" + flag + "_" + root.ch
    }

    // Read the applied flags + the accents list. Runs on show and after apply
    // so the UI always reflects what's actually on disk.
    function refresh() {
        SystemSettingsManager.readBatch([
            root.flagPath("custom_acc"),
            root.flagPath("acc_from_wall"),
            root.flagPath("acc_from_hex"),
            root.flagPath("custom_acc_name"),
            root.flagPath("defined_hex"),
            root.flagPath("custom_acc_hyprland"),
            SystemSettingsManager.hyprCache + "/launcher_cache/custom_acc.json"
        ], r => {
            const custom = String(r[0] || "").trim()
            const wall = String(r[1] || "").trim()
            const hex = String(r[2] || "").trim()
            root.activeSource = custom === "1" ? "custom"
                : wall === "1" ? "wall"
                : hex === "1" ? "hex"
                : "theme"
            root.activeCustomName = String(r[3] || "").trim()
            root.activeHex = String(r[4] || "").trim()
            root.selSource = root.activeSource
            root.selCustomName = root.activeCustomName
            root.selHex = ""
            root.hyprlandToggle = String(r[6] || "0").trim() === "1"
            const raw = String(r[5] || "").trim()
            if (!raw)
                return
            try {
                root.accents = JSON.parse(raw)
            } catch (e) {
                console.log("[appearance] accents parse failed: " + e)
            }
            for (let i = 0; i < alphaRows.count; i++)
                alphaRows.itemAt(i).readCur()
        })
    }

    // Hand the accent source to theme_main — QML writes NO flags anymore.
    // theme_main accent <key> [hex]: theme_default | acc_from_wall |
    // define_hex <hex> | custom_acc <name>.
    function applyAccent(scope) {
        // 'all' is a valid action even when the current channel already uses
        // this source (isDirty only reflects the active channel).
        if (!root.isDirty && scope !== "all")
            return
        let arg = "theme_default"
        if (root.selSource === "wall")
            arg = scope === "all" ? "acc_from_wall all" : "acc_from_wall"
        else if (root.selSource === "hex")
            arg = "define_hex '" + root.resolvedHex + "'"
        else if (root.selSource === "custom")
            arg = "custom_acc '" + root.selCustomName + "'"
        SystemSettingsManager.apply("theme_main", "accent " + arg)
    }

    // ----- layout --------------------------------------------------------

    Column {
        id: col
        width: parent.width
        spacing: 12

        // --- Accent Color ---
        Txt {
            text: "Accent Color"
            color: Theme.fg3
            font.family: Theme.fontName
            font.pixelSize: 11
        }

        Repeater {
            model: [
                { key: "theme", title: "Theme Default", hint: "Use the scheme's own accent" },
                { key: "wall", title: "Accent from Wallpaper", hint: "Extract the accent from the wallpaper" },
                { key: "hex", title: "Defined Hex", hint: "Apply a specific hex color" },
                { key: "custom", title: "Custom Accent", hint: "Pick from the curated accent list" }
            ]

            delegate: Rectangle {
                required property int index
                required property var modelData
                readonly property bool isSel: root.selSource === modelData.key
                width: col.width
                height: 46
                radius: 10
                color: isSel ? Theme.acc : Theme.bg2
                border.width: isSel ? 0 : 1
                border.color: Theme.bg3

                Behavior on color {
                    ColorAnimation { duration: 120 }
                }

                Row {
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    spacing: 10

                    Rectangle {
                        id: dot
                        anchors.verticalCenter: parent.verticalCenter
                        width: 14
                        height: 14
                        radius: 7
                        color: isSel ? Theme.sfg : Theme.fg3
                        border.width: 1
                        border.color: isSel ? "transparent" : Theme.bg3
                    }

                    Column {
                        anchors.verticalCenter: parent.verticalCenter
                        width: parent.width - dot.width - 10

                        Txt {
                            text: modelData.title
                            color: isSel ? Theme.sfg : Theme.fg
                            font.family: Theme.fontName
                            font.pixelSize: 13
                            font.bold: true
                        }

                        Txt {
                            text: modelData.hint
                            color: isSel ? Theme.sfg : Theme.fg3
                            font.family: Theme.fontName
                            font.pixelSize: 10
                            opacity: isSel ? 0.85 : 1
                        }
                    }
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.selSource = modelData.key
                }
            }
        }

        // --- Defined hex (visible when hex selected) ---
        Column {
            width: parent.width
            spacing: 8
            visible: root.selSource === "hex"

            Rectangle {
                width: parent.width
                height: 40
                radius: 10
                color: Theme.bg2
                border.width: 1
                border.color: Theme.bg3

                Row {
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    spacing: 10

                    Rectangle {
                        anchors.verticalCenter: parent.verticalCenter
                        width: 24
                        height: 24
                        radius: 12
                        border.width: 1
                        border.color: Theme.bg3
                        color: root.resolvedHex || "#888888"
                    }

                    Txt {
                        anchors.verticalCenter: parent.verticalCenter
                        text: root.resolvedHex || "No hex picked yet"
                        color: Theme.fg
                        font.family: Theme.fontName
                        font.pixelSize: 12
                    }

                    Item { width: 1; height: 1 }

                    Txt {
                        anchors.verticalCenter: parent.verticalCenter
                        text: "\uf1fb"
                        color: Theme.fg3
                        font.family: Theme.iconFont
                        font.pixelSize: 13
                    }

                    Txt {
                        anchors.verticalCenter: parent.verticalCenter
                        text: "Pick from screen"
                        color: Theme.fg
                        font.family: Theme.fontName
                        font.pixelSize: 12
                    }
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        SystemSettingsManager.readLines("hyprpicker -a", lines => {
                            const hex = String(lines[0] || "").trim().toUpperCase()
                            if (hex.length === 7)
                                root.selHex = hex
                        })
                    }
                }
            }
        }

        // --- Custom accent list (visible when custom selected) ---
        Column {
            width: parent.width
            spacing: 6
            visible: root.selSource === "custom"

            Txt {
                text: "Pick an accent:"
                color: Theme.fg3
                font.family: Theme.fontName
                font.pixelSize: 11
            }

            Rectangle {
                width: parent.width
                height: 170
                radius: 10
                color: Theme.bg2
                clip: true

                GridView {
                    id: accGrid
                    anchors.fill: parent
                    anchors.margins: 6
                    clip: true
                    model: root.accents
                    cellWidth: Math.floor(accGrid.width / 5)
                    cellHeight: 56

                    delegate: Item {
                        required property var modelData
                        readonly property bool isSel: root.selCustomName === modelData.name
                        width: accGrid.cellWidth
                        height: accGrid.cellHeight

                        Rectangle {
                            id: swatch
                            anchors.top: parent.top
                            anchors.horizontalCenter: parent.horizontalCenter
                            width: 34
                            height: 34
                            radius: 10
                            color: SystemSettingsManager.mode === "l" ? (modelData.l || modelData.d) : modelData.d
                            border.width: isSel ? 2 : 1
                            border.color: isSel ? Theme.acc : Theme.bg3

                            Rectangle {
                                anchors.fill: parent
                                radius: 10
                                color: isSel ? "#40000000" : "transparent"
                            }

                            Txt {
                                anchors.centerIn: parent
                                visible: isSel
                                text: "\uf00c"
                                font.family: Theme.iconFont
                                font.pixelSize: 14
                                color: Theme.sfg
                            }

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    root.selCustomName = modelData.name
                                    root.selSource = "custom"
                                }
                            }
                        }

                        Txt {
                            anchors.top: swatch.bottom
                            anchors.topMargin: 4
                            anchors.horizontalCenter: parent.horizontalCenter
                            width: accGrid.cellWidth - 8
                            text: modelData.name
                            color: isSel ? Theme.fg : Theme.fg3
                            font.family: Theme.fontName
                            font.pixelSize: 10
                            horizontalAlignment: Text.AlignHCenter
                            elide: Text.ElideRight
                        }
                    }
                }

                ScrollDragger {
                    target: accGrid
                    thumbWidth: 4
                    minThumbHeight: 26
                }
            }
        }

        // --- Apply: wall scope choice (wall option gets two buttons) ---
        Column {
            width: parent.width
            spacing: 6
            visible: root.selSource === "wall"

            Rectangle {
                width: parent.width
                height: 40
                radius: 10
                color: root.isDirty ? Theme.acc : Theme.bg3
                opacity: root.isDirty ? 1 : 0.5

                Behavior on color {
                    ColorAnimation { duration: 120 }
                }

                Txt {
                    anchors.centerIn: parent
                    text: "Apply to this channel"
                    color: root.isDirty ? Theme.sfg : Theme.fg3
                    font.family: Theme.fontName
                    font.pixelSize: 13
                    font.bold: true
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    enabled: root.isDirty
                    onClicked: root.applyAccent("channel")
                }
            }

            Rectangle {
                width: parent.width
                height: 40
                radius: 10
                color: Theme.acc

                Behavior on color {
                    ColorAnimation { duration: 120 }
                }

                Txt {
                    anchors.centerIn: parent
                    text: "Apply to all channels"
                    color: Theme.sfg
                    font.family: Theme.fontName
                    font.pixelSize: 13
                    font.bold: true
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.applyAccent("all")
                }
            }
        }

        // --- Apply button (theme / hex / custom) ---
        Rectangle {
            width: parent.width
            height: 40
            radius: 10
            color: root.isDirty ? Theme.acc : Theme.bg3
            opacity: root.isDirty ? 1 : 0.5
            visible: root.selSource !== "wall"

            Behavior on color {
                ColorAnimation { duration: 120 }
            }

            Txt {
                anchors.centerIn: parent
                text: "Apply"
                color: root.isDirty ? Theme.sfg : Theme.fg3
                font.family: Theme.fontName
                font.pixelSize: 13
                font.bold: true
            }

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                enabled: root.isDirty
                onClicked: root.applyAccent()
            }
        }

        Rectangle {
            width: parent.width
            height: 1
            color: Theme.bg3
        }

        // --- Alpha & Tint (dropdowns) ---
        Txt {
            text: "Alpha & Tint"
            color: Theme.fg3
            font.family: Theme.fontName
            font.pixelSize: 11
        }

        Repeater {
            id: alphaRows

            model: [
                { flag: "acc_alpha", cmd: "acc_alpha", label: "Accent Alpha", hint: "(default is bf)", viaMain: true },
                { flag: "border_alpha", cmd: "border_alpha", label: "Border Alpha", hint: "(default is bf)", viaMain: true }
            ]

            delegate: Column {
                required property var modelData
                id: arow

                readonly property string flag: modelData.flag
                readonly property string cmdFlag: modelData.cmd
                readonly property bool viaMain: modelData.viaMain
                readonly property string label: modelData.label
                readonly property string hint: modelData.hint

                property string cur: ""
                property bool open: false
                property int sel: -1
                readonly property var entries: [
                    { l: "25% · 0x40", v: "40" },
                    { l: "40% · 0x66", v: "66" },
                    { l: "55% · 0x8c", v: "8c" },
                    { l: "70% · 0xb2", v: "b2" },
                    { l: "85% · 0xd8", v: "d8" },
                    { l: "100% · 0xff", v: "ff" }
                ]

                width: parent.width
                spacing: 6

                function flagPath() {
                    return SystemSettingsManager.states + "/" + arow.flag + "_" + root.ch
                }

                function readCur() {
                    SystemSettingsManager.readBatch([arow.flagPath()], r => {
                        arow.cur = String(r[0] || "").trim()
                    })
                }

                function commit() {
                    const v = arow.entries[arow.sel].v
                    if (arow.viaMain)
                        SystemSettingsManager.apply("alpha_main", arow.cmdFlag + " " + v)
                    else
                        SystemSettingsManager.writeFiles([{ path: arow.flagPath(), value: v }])
                    arow.cur = v
                    arow.open = false
                }

                Rectangle {
                    width: parent.width
                    height: 46
                    radius: 10
                    color: arow.open ? Theme.acc : Theme.bg2
                    border.width: arow.open ? 0 : 1
                    border.color: Theme.bg3

                    Behavior on color {
                        ColorAnimation { duration: 120 }
                    }

                    Column {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.leftMargin: 12
                        anchors.rightMargin: 12
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 2

                        Txt {
                            width: parent.width
                            text: arow.label
                            color: arow.open ? Theme.sfg : Theme.fg
                            font.family: Theme.fontName
                            font.pixelSize: 13
                            font.bold: true
                            elide: Text.ElideRight
                        }

                        Txt {
                            width: parent.width
                            text: arow.hint
                            color: arow.open ? Theme.sfg : Theme.fg3
                            font.family: Theme.fontName
                            font.pixelSize: 10
                            elide: Text.ElideRight
                        }
                    }

                    Row {
                        anchors.right: parent.right
                        anchors.rightMargin: 12
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 8

                        Txt {
                            anchors.verticalCenter: parent.verticalCenter
                            text: arow.cur || "not set"
                            color: arow.open ? Theme.sfg : Theme.fg
                            font.family: Theme.fontName
                            font.pixelSize: 12
                        }

                        Txt {
                            anchors.verticalCenter: parent.verticalCenter
                            text: arow.open ? "\uf0d8" : "\uf0d7"
                            font.family: Theme.iconFont
                            font.pixelSize: 11
                            color: arow.open ? Theme.sfg : Theme.fg3
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            arow.open = !arow.open
                            arow.forceActiveFocus()
                        }
                    }
                }

                Rectangle {
                    width: parent.width
                    height: arow.open ? Math.min(224, (arow.entries.length * 34) + 8) : 0
                    radius: 10
                    color: Theme.bg2
                    border.width: 1
                    border.color: Theme.bg3
                    visible: arow.open
                    clip: true

                    ListView {
                        id: alphaList
                        anchors.fill: parent
                        anchors.margins: 4
                        clip: true
                        interactive: false
                        model: arow.entries

                        delegate: Rectangle {
                            required property int index
                            required property var modelData
                            width: alphaList.width
                            height: 34
                            radius: 7
                            color: index === arow.sel ? Theme.acc
                                : (amouse.containsMouse ? Theme.hover : "transparent")

                            Behavior on color {
                                ColorAnimation { duration: 100 }
                            }

                            Txt {
                                anchors.left: parent.left
                                anchors.leftMargin: 12
                                anchors.right: parent.right
                                anchors.rightMargin: 8
                                anchors.verticalCenter: parent.verticalCenter
                                text: modelData.l
                                color: index === arow.sel ? Theme.sfg : Theme.fg
                                font.family: Theme.fontName
                                font.pixelSize: 12
                            }

                            MouseArea {
                                id: amouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onEntered: arow.sel = index
                                onClicked: {
                                    arow.sel = index
                                    arow.commit()
                                }
                            }
                        }
                    }

                    ScrollDragger {
                        target: alphaList
                        thumbWidth: 4
                        minThumbHeight: 24
                    }
                }

                Keys.onPressed: event => {
                    if (!arow.open)
                        return
                    if (event.key === Qt.Key_Escape) {
                        arow.open = false
                        event.accepted = true
                    } else if (event.key === Qt.Key_Up) {
                        arow.sel = (arow.sel - 1 + arow.entries.length) % arow.entries.length
                        event.accepted = true
                    } else if (event.key === Qt.Key_Down) {
                        arow.sel = (arow.sel + 1) % arow.entries.length
                        event.accepted = true
                    } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                        arow.commit()
                        event.accepted = true
                    }
                }

                Component.onCompleted: arow.readCur()
            }
        }

        // --- Accent Toggles ---
        Txt {
            text: "Accent Toggles"
            color: Theme.fg3
            font.family: Theme.fontName
            font.pixelSize: 11
        }

        Rectangle {
            width: parent.width
            height: 46
            radius: 10
            color: Theme.bg2
            border.width: 1
            border.color: Theme.bg3

            Column {
                anchors.left: parent.left
                anchors.right: hyprToggle.left
                anchors.rightMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2

                Txt {
                    text: "Hyprland Window Borders"
                    color: Theme.fg
                    font.family: Theme.fontName
                    font.pixelSize: 13
                    font.bold: true
                    elide: Text.ElideRight
                }

                Txt {
                    width: parent.width
                    text: "(default is enabled)"
                    color: Theme.fg3
                    font.family: Theme.fontName
                    font.pixelSize: 10
                    wrapMode: Text.WordWrap
                }
            }

            ToggleSwitch {
                id: hyprToggle
                anchors.right: parent.right
                anchors.rightMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                checked: root.hyprlandToggle
                onToggled: value => {
                    root.hyprlandToggle = value
                    SystemSettingsManager.writeFiles([{ path: root.flagPath("custom_acc_hyprland"), value: value ? "1" : "0" }])
                }
            }
        }

        // --- Font Family ---
        Txt {
            text: "Font Family"
            color: Theme.fg3
            font.family: Theme.fontName
            font.pixelSize: 11
        }

        Rectangle {
            width: parent.width
            height: 42
            radius: 10
            color: Theme.bg2
            border.width: 1
            border.color: fontInput.activeFocus ? Theme.acc : Theme.bg3

            TxtInput {
                id: fontInput
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                verticalAlignment: TextInput.AlignVCenter
                color: Theme.fg
                font.family: Theme.fontName
                font.pixelSize: 13
                text: SettingsManager.config.appearance.font
                selectByMouse: true

                onEditingFinished: {
                    const value = fontInput.text.trim()
                    if (value.length > 0)
                        SettingsManager.set("appearance", "font", value)
                    else
                        fontInput.text = SettingsManager.config.appearance.font
                }
            }
        }

        // --- Icon Theme ---
        Txt {
            text: "Icon Theme"
            color: Theme.fg3
            font.family: Theme.fontName
            font.pixelSize: 11
        }

        Rectangle {
            width: parent.width
            height: 42
            radius: 10
            color: Theme.bg2
            border.width: 1
            border.color: iconThemeInput.activeFocus ? Theme.acc : Theme.bg3

            TxtInput {
                id: iconThemeInput
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                verticalAlignment: TextInput.AlignVCenter
                color: Theme.fg
                font.family: Theme.fontName
                font.pixelSize: 13
                text: SettingsManager.config.appearance.iconTheme
                selectByMouse: true

                onEditingFinished: {
                    const value = iconThemeInput.text.trim()
                    if (value.length > 0)
                        SettingsManager.set("appearance", "iconTheme", value)
                    else
                        iconThemeInput.text = SettingsManager.config.appearance.iconTheme
                }
            }
        }
    }

    ScrollDragger {
        target: root
        thumbWidth: 4
        minThumbHeight: 26
    }

    Connections {
        target: SystemSettingsManager
        function onChannelChanged() { root.refresh() }
        function onModeChanged() { root.refresh() }
        function onApplied() { root.refresh() }
    }

    onVisibleChanged: {
        if (visible)
            root.refresh()
    }

    Component.onCompleted: root.refresh()
}
