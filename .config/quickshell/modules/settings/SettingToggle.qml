import QtQuick
import "../../components"
import "../../services"

// Toggle row for a SettingSection: label + hint on the left, an iOS-style
// ToggleSwitch on the right. Emits through engine.setValue() so the engine
// writes the state file and runs the row's apply script. Dims when the
// schema marks the row disabled (e.g. the eye-care temperature slider
// while the filter is not in manual mode).
Item {
    id: root

    property var row: null
    property var engine: null

    implicitHeight: 46
    width: parent ? parent.width : 0

    opacity: root.row && root.row.enabled ? 1 : 0.45
    Behavior on opacity { NumberAnimation { duration: 120 } }

    Column {
        anchors.left: parent.left
        anchors.right: toggle.left
        anchors.rightMargin: 12
        anchors.verticalCenter: parent.verticalCenter
        spacing: 2

        Txt {
            text: root.row ? root.row.label : ""
            color: Theme.fg
            font.family: Theme.fontName
            font.pixelSize: 13
            font.bold: true
            elide: Text.ElideRight
        }

        Txt {
            visible: root.row && root.row.hint.length > 0
            width: parent.width
            wrapMode: Text.WordWrap
            text: root.row ? root.row.hint : ""
            color: Theme.fg3
            font.family: Theme.fontName
            font.pixelSize: 11
        }
    }

    ToggleSwitch {
        id: toggle
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        checked: root.row ? root.row.value === "1" : false
        enabled: root.row ? root.row.enabled : false
        onToggled: value => {
            if (root.engine && root.row)
                root.engine.setValue(root.row.key, value)
        }
    }
}
