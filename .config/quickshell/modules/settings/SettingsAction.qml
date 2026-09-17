import QtQuick
import "../../components"
import "../../services"

// Full-width action row used by the Clipboard / Restore categories: glyph,
// label + hint, chevron. Mirrors the SettingButton visual language but
// wired directly to QML-side handlers instead of the schema engine.
Rectangle {
    id: root

    property string label: ""
    property string glyph: ""
    property string hint: ""
    signal clicked()

    width: 340
    height: 44
    radius: 12
    color: rootMouse.containsMouse ? Theme.hover : Theme.bg2
    border.width: 1
    border.color: Theme.bg3

    Behavior on color { ColorAnimation { duration: 120 } }

    Txt {
        anchors.left: parent.left
        anchors.leftMargin: 14
        anchors.verticalCenter: parent.verticalCenter
        text: root.glyph
        font.family: Theme.fontName
        font.pixelSize: 13
        color: Theme.fg2
    }

    Column {
        anchors.left: parent.left
        anchors.leftMargin: 42
        anchors.right: parent.right
        anchors.rightMargin: 12
        anchors.verticalCenter: parent.verticalCenter
        spacing: 1

        Txt {
            text: root.label
            color: Theme.fg
            font.family: Theme.fontName
            font.pixelSize: 12
            font.bold: true
            elide: Text.ElideRight
        }

        Txt {
            visible: root.hint.length > 0
            text: root.hint
            color: Theme.fg3
            font.family: Theme.fontName
            font.pixelSize: 10
            elide: Text.ElideRight
        }
    }

    Txt {
        anchors.right: parent.right
        anchors.rightMargin: 12
        anchors.verticalCenter: parent.verticalCenter
        text: "\uf054"
        font.family: Theme.iconFont
        font.pixelSize: 10
        color: Theme.fg3
    }

    MouseArea {
        id: rootMouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.clicked()
    }
}
