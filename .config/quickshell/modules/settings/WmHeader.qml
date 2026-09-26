import QtQuick
import "../../components"

// Section header inside WmCategory: an accent tick + uppercase label.
Item {
    id: root

    property string label: ""

    implicitHeight: 28
    width: parent ? parent.width : 0

    Rectangle {
        width: 3
        height: 12
        radius: 1.5
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        color: Theme.acc
    }

    Txt {
        anchors.left: parent.left
        anchors.leftMargin: 10
        anchors.verticalCenter: parent.verticalCenter
        text: root.label
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 11
        font.bold: true
        textFormat: Text.PlainText
    }
}
