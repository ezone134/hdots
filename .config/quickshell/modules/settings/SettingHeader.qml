import QtQuick
import "../../components"
import "../../services"

// Section header inside a SettingSection: an accent tick + uppercase
// label. Rendered by SettingItem for rows of type "header" — pure
// decoration, no interaction.
Item {
    id: root

    property var row: null
    property var engine: null

    implicitHeight: 30
    width: parent ? parent.width : 0

    Row {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        spacing: 8

        Rectangle {
            width: 3
            height: 14
            radius: 1.5
            anchors.verticalCenter: parent.verticalCenter
            color: Theme.acc
        }

        Txt {
            anchors.verticalCenter: parent.verticalCenter
            text: (root.row ? root.row.label : "").toUpperCase()
            color: Theme.fg2
            font.family: Theme.fontName
            font.pixelSize: 11
            font.bold: true
            elide: Text.ElideRight
        }
    }
}
