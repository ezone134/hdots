import QtQuick
import "../../components"

// Toggle row for WmCategory: label + hint on the left, an iOS-style
// ToggleSwitch on the right. Emits changed(value) so the category can write
// the channel-scoped state file and run the restore.
Item {
    id: root

    property string label: ""
    property string hint: ""
    property bool value: false
    signal changed(bool value)

    implicitHeight: root.hint.length > 0 ? 52 : 40
    width: parent ? parent.width : 0

    Column {
        anchors.left: parent.left
        anchors.right: toggle.left
        anchors.rightMargin: 12
        anchors.verticalCenter: parent.verticalCenter
        spacing: 2

        Txt {
            text: root.label
            color: Theme.fg
            font.family: Theme.fontName
            font.pixelSize: 13
            font.bold: true
            elide: Text.ElideRight
        }

        Txt {
            visible: root.hint.length > 0
            width: parent.width
            wrapMode: Text.WordWrap
            text: root.hint
            color: Theme.fg3
            font.family: Theme.fontName
            font.pixelSize: 11
        }
    }

    ToggleSwitch {
        id: toggle
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        checked: root.value
        onToggled: value => root.changed(value)
    }
}
