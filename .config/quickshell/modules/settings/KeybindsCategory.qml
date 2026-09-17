import QtQuick
import "../../components"
import "../../services"

// Keybinds category (mirrors "Show Keybinds Shortcut"): a searchable,
// read-only viewer for the keybind list the dotfiles cache at
// $hypr_cache/keybinds. No rofi involved.
Column {
    id: root
    anchors.fill: parent
    spacing: 10

    property var entries: []
    property string filter: ""

    readonly property var filteredEntries: {
        const f = root.filter.trim().toLowerCase()
        if (!f)
            return root.entries
        const out = []
        for (let i = 0; i < root.entries.length; i++) {
            if (root.entries[i].toLowerCase().indexOf(f) >= 0)
                out.push(root.entries[i])
        }
        return out
    }

    function reload() {
        SystemSettingsManager.readBatch([SystemSettingsManager.hyprCache + "/keybinds"], r => {
            const raw = String(r[0] || "")
            const out = []
            const parts = raw.split("\n")
            for (let i = 0; i < parts.length; i++) {
                const t = parts[i].replace(/\\n/g, " ").replace(/\\r/g, "").trim()
                if (t)
                    out.push(t)
            }
            root.entries = out
        })
    }

    Row {
        width: parent.width
        spacing: 10

        Txt {
            anchors.verticalCenter: parent.verticalCenter
            text: "\uf11c  Hyprland Keybinds"
            color: Theme.fg
            font.family: Theme.fontName
            font.pixelSize: 14
            font.bold: true
        }

        Txt {
            anchors.verticalCenter: parent.verticalCenter
            text: String(root.entries.length) + " binds"
            color: Theme.fg3
            font.family: Theme.fontName
            font.pixelSize: 11
        }
    }

    Rectangle {
        width: parent.width
        height: 38
        radius: 10
        color: Theme.bg2
        border.width: 1
        border.color: kbFilter.activeFocus ? Theme.acc : Theme.bg3

        Txt {
            anchors.left: parent.left
            anchors.leftMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            text: "\uf002"
            font.family: Theme.fontName
            font.pixelSize: 12
            color: Theme.fg3
        }

        TxtInput {
            id: kbFilter
            anchors.left: parent.left
            anchors.leftMargin: 32
            anchors.right: parent.right
            anchors.rightMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            color: Theme.fg
            font.family: Theme.fontName
            font.pixelSize: 12
            selectByMouse: true
            onTextChanged: root.filter = text
        }

        // Quickshell's compiler rejects placeholderText, so show a grey
        // hint that disappears once the user types.
        Txt {
            anchors.left: parent.left
            anchors.leftMargin: 32
            anchors.right: parent.right
            anchors.rightMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            visible: kbFilter.text.length === 0 && !kbFilter.activeFocus
            text: "Filter keybinds..."
            color: Theme.fg3
            font.family: Theme.fontName
            font.pixelSize: 12
        }
    }

    Rectangle {
        width: parent.width
        height: parent.height - 96
        radius: 12
        color: Theme.bg2
        clip: true

        ListView {
            id: kbList
            anchors.fill: parent
            anchors.margins: 6
            clip: true
            spacing: 2
            model: root.filteredEntries
            boundsBehavior: Flickable.StopAtBounds

            delegate: Rectangle {
                required property string modelData
                width: kbList.width
                height: 30
                radius: 8
                color: kbm.containsMouse ? Theme.hover : "transparent"

                Txt {
                    anchors.left: parent.left
                    anchors.leftMargin: 10
                    anchors.right: parent.right
                    anchors.rightMargin: 10
                    anchors.verticalCenter: parent.verticalCenter
                    text: modelData
                    color: Theme.fg
                    font.family: Theme.fontName
                    font.pixelSize: 11
                    elide: Text.ElideRight
                }

                MouseArea {
                    id: kbm
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                }
            }
        }

        ScrollDragger {
            target: kbList
            thumbWidth: 5
            minThumbHeight: 28
        }

        EmptyState {
            anchors.centerIn: parent
            visible: root.entries.length === 0
            icon: "\uf11c"
            text: "Keybinds cache is empty — generate it from the dotfiles first"
        }
    }

    Component.onCompleted: root.reload()
}
