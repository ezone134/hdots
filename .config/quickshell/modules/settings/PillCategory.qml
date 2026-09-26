import QtQuick
import "../../components"
import "../../services"

// Pill category: collapsed chips + reorder. Clock is always on (reorder only).
// Persisted under $states/settings.json "pill" via SettingsManager.set().
Column {
    id: root
    spacing: 12

    readonly property var catalog: [
        { key: "clock", title: "Clock", desc: "Time on the collapsed island. Always on — reorder only.", defTrue: true, locked: true },
        { key: "musicBarAnim", title: "Music playing animation", desc: "Equalizer on the collapsed island while playing. Expanded always shows it.", defTrue: true, locked: false },
        { key: "collapsedBattery", title: "Battery", desc: "Icon and level when collapsed.", defTrue: false, locked: false },
        { key: "collapsedWatts", title: "Wattage", desc: "Power draw in watts when collapsed.", defTrue: false, locked: false },
        { key: "collapsedWeather", title: "Weather", desc: "Condition glyph and temperature.", defTrue: false, locked: false },
        { key: "collapsedNotification", title: "Notifications", desc: "Bell that opens the notification center.", defTrue: false, locked: false },
        { key: "collapsedWorkspace", title: "Workspaces", desc: "Shortcut to the workspace switcher.", defTrue: false, locked: false },
        { key: "collapsedSettings", title: "Settings", desc: "Gear shortcut back to this app.", defTrue: false, locked: false },
        { key: "collapsedPower", title: "Power menu", desc: "Session / power menu shortcut.", defTrue: false, locked: false }
    ]

    readonly property var defaultOrder: [
        "clock", "musicBarAnim", "collapsedBattery", "collapsedWatts", "collapsedWeather",
        "collapsedNotification", "collapsedWorkspace", "collapsedSettings", "collapsedPower"
    ]

    function metaFor(key) {
        for (let i = 0; i < root.catalog.length; i++) {
            if (root.catalog[i].key === key)
                return root.catalog[i]
        }
        return null
    }

    function resolvedOrder() {
        const raw = (SettingsManager.config.pill && SettingsManager.config.pill.itemOrder) || []
        const out = []
        const seen = {}
        for (let i = 0; i < raw.length; i++) {
            const k = String(raw[i] || "")
            if (!k || seen[k] || !root.metaFor(k))
                continue
            seen[k] = true
            out.push(k)
        }
        for (let j = 0; j < root.defaultOrder.length; j++) {
            const d = root.defaultOrder[j]
            if (!seen[d])
                out.push(d)
        }
        // Clock must always appear once.
        if (out.indexOf("clock") < 0)
            out.unshift("clock")
        return out
    }

    function rebuildModel() {
        orderModel.clear()
        const keys = root.resolvedOrder()
        for (let i = 0; i < keys.length; i++) {
            const m = root.metaFor(keys[i])
            if (!m)
                continue
            orderModel.append({
                key: m.key,
                title: m.title,
                desc: m.desc,
                defTrue: m.defTrue,
                locked: m.locked === true
            })
        }
    }

    function persistOrder() {
        const keys = []
        for (let i = 0; i < orderModel.count; i++)
            keys.push(orderModel.get(i).key)
        if (keys.indexOf("clock") < 0)
            keys.unshift("clock")
        SettingsManager.set("pill", "itemOrder", keys)
    }

    function moveRow(from, to) {
        if (from === to || from < 0 || to < 0 || from >= orderModel.count || to >= orderModel.count)
            return
        orderModel.move(from, to, 1)
        root.persistOrder()
    }

    ListModel {
        id: orderModel
    }

    Connections {
        target: SettingsManager
        function onConfigChanged() {
            const cur = []
            for (let i = 0; i < orderModel.count; i++)
                cur.push(orderModel.get(i).key)
            const next = root.resolvedOrder()
            if (cur.join("|") !== next.join("|"))
                root.rebuildModel()
        }
    }

    Component.onCompleted: root.rebuildModel()

    Txt {
        text: "Collapsed bar items"
        color: Theme.fg
        font.family: Theme.fontName
        font.pixelSize: 13
        font.bold: true
    }

    Txt {
        width: 440
        wrapMode: Text.WordWrap
        text: "Reorder with ↑ ↓. Clock stays on. Other chips can be toggled — same order as the collapsed island."
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 11
    }

    Column {
        width: 440
        spacing: 4

        Repeater {
            model: orderModel

            delegate: Rectangle {
                id: rowCard
                required property int index
                required property string key
                required property string title
                required property string desc
                required property bool defTrue
                required property bool locked

                width: 440
                height: 52
                radius: 10
                color: Theme.bg2
                border.width: 1
                border.color: Theme.bg3

                Column {
                    id: movers
                    anchors.left: parent.left
                    anchors.leftMargin: 8
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 2

                    Rectangle {
                        width: 22
                        height: 18
                        radius: 6
                        color: upMa.containsMouse ? Theme.hover : "transparent"
                        opacity: rowCard.index > 0 ? 1 : 0.35

                        Txt {
                            anchors.centerIn: parent
                            text: "\uf077"
                            font.family: Theme.iconFont
                            font.pixelSize: 11
                            color: Theme.fg
                        }
                        MouseArea {
                            id: upMa
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            enabled: rowCard.index > 0
                            onClicked: root.moveRow(rowCard.index, rowCard.index - 1)
                        }
                    }

                    Rectangle {
                        width: 22
                        height: 18
                        radius: 6
                        color: downMa.containsMouse ? Theme.hover : "transparent"
                        opacity: rowCard.index < orderModel.count - 1 ? 1 : 0.35

                        Txt {
                            anchors.centerIn: parent
                            text: "\uf078"
                            font.family: Theme.iconFont
                            font.pixelSize: 11
                            color: Theme.fg
                        }
                        MouseArea {
                            id: downMa
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            enabled: rowCard.index < orderModel.count - 1
                            onClicked: root.moveRow(rowCard.index, rowCard.index + 1)
                        }
                    }
                }

                Column {
                    anchors.left: movers.right
                    anchors.leftMargin: 10
                    anchors.right: trailing.left
                    anchors.rightMargin: 12
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 2

                    Txt {
                        text: rowCard.title
                        color: Theme.fg
                        font.family: Theme.fontName
                        font.pixelSize: 13
                        font.bold: true
                    }

                    Txt {
                        width: parent.width
                        wrapMode: Text.WordWrap
                        text: rowCard.desc
                        color: Theme.fg3
                        font.family: Theme.fontName
                        font.pixelSize: 11
                    }
                }

                Item {
                    id: trailing
                    anchors.right: parent.right
                    anchors.rightMargin: 10
                    anchors.verticalCenter: parent.verticalCenter
                    width: rowCard.locked ? lockedLbl.implicitWidth : switchControl.width
                    height: 28

                    Txt {
                        id: lockedLbl
                        visible: rowCard.locked
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.right: parent.right
                        text: "Always on"
                        color: Theme.fg3
                        font.family: Theme.fontName
                        font.pixelSize: 11
                        font.bold: true
                    }

                    ToggleSwitch {
                        id: switchControl
                        visible: !rowCard.locked
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        checked: rowCard.defTrue
                            ? !(SettingsManager.config.pill && SettingsManager.config.pill[rowCard.key] === false)
                            : (SettingsManager.config.pill && SettingsManager.config.pill[rowCard.key] === true)
                        onToggled: value => {
                            if (!rowCard.locked)
                                SettingsManager.set("pill", rowCard.key, value)
                        }
                    }
                }
            }
        }
    }
}
