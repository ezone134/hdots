import QtQuick
import "../services"

// Round/square icon chip with hover, optional selected state and left/
// right click signals. Basis for panel header bubbles and pill icons.
Item {
    id: root

    property string glyph: ""
    property string family: Theme.iconFont
    property int size: 24
    property int glyphSize: 14
    property color color: Theme.fg
    property color fill: "transparent"
    property color selectedFill: Theme.acc
    property color selectedColor: Theme.sfg
    property bool selected: false
    property int radius: size / 2
    property int borderWidth: 0
    property color borderColor: "transparent"

    signal clicked()
    signal rightClicked()

    implicitWidth: size
    implicitHeight: size

    Rectangle {
        anchors.fill: parent
        radius: root.radius
        color: root.selected ? root.selectedFill : root.fill
        border.width: root.borderWidth
        border.color: root.borderColor

        Behavior on color {
            ColorAnimation { duration: 120 }
        }

        Txt {
            anchors.centerIn: parent
            text: root.glyph
            font.family: root.family
            font.pixelSize: root.glyphSize
            color: root.selected ? root.selectedColor : root.color
        }
    }

    MouseArea {
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: mouse => {
            if (mouse.button === Qt.RightButton)
                root.rightClicked()
            else
                root.clicked()
        }
    }
}
