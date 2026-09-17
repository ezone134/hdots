import QtQuick
import "../../components"
import "../../services"

// Action button row for a SettingSection (e.g. "Use theme default accent"):
// a full-width outlined button. Clicking commits the row through
// engine.setValue() — for button-type rows the engine skips value writes
// and only fires the row's extra writes + apply script.
Item {
    id: root

    property var row: null
    property var engine: null

    implicitHeight: 42
    width: parent ? parent.width : 0

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        height: 42
        radius: 12
        color: btnMouse.containsMouse ? Theme.hover : Theme.bg2
        border.width: 1
        border.color: Theme.bg3

        Behavior on color {
            ColorAnimation { duration: 120 }
        }

        Txt {
            anchors.left: parent.left
            anchors.leftMargin: 16
            anchors.right: chevronGlyph.left
            anchors.rightMargin: 8
            anchors.verticalCenter: parent.verticalCenter
            text: root.row ? root.row.label : ""
            color: Theme.fg
            font.family: Theme.fontName
            font.pixelSize: 12
            font.bold: true
            elide: Text.ElideRight
        }

        Txt {
            id: chevronGlyph
            anchors.right: parent.right
            anchors.rightMargin: 14
            anchors.verticalCenter: parent.verticalCenter
            text: "\uf054"
            font.family: Theme.iconFont
            font.pixelSize: 10
            color: Theme.fg3
        }

        MouseArea {
            id: btnMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: {
                if (root.engine && root.row)
                    root.engine.setValue(root.row.key, true)
            }
        }
    }
}
