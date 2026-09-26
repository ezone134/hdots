import QtQuick
import "../services"

// Panel header: optional icon bubble + title + subtitle. Left-aligned;
// the caller positions it (e.g. anchors.horizontalCenter for centered
// titles with no icon).
Item {
    id: root

    property string icon: ""
    property string title: ""
    property string subtitle: ""
    property int bubbleSize: 38
    property color bubbleColor: Theme.bg3
    property string iconFamily: Theme.iconFont
    property int titleSize: 15

    implicitHeight: bubbleSize
    implicitWidth: contentRow.implicitWidth

    Row {
        id: contentRow
        anchors.left: parent.left
        anchors.right: parent.right
        spacing: 12

        Rectangle {
            width: root.bubbleSize
            height: root.bubbleSize
            radius: root.bubbleSize / 2
            color: root.bubbleColor
            visible: root.icon.length > 0

            Txt {
                anchors.centerIn: parent
                text: root.icon
                font.family: root.iconFamily
                font.pixelSize: Math.round(root.bubbleSize / 2)
                color: Theme.fg
            }
        }

        Column {
            anchors.verticalCenter: parent.verticalCenter
            spacing: 1

            Txt {
                text: root.title
                color: Theme.fg
                font.family: Theme.fontName
                font.pixelSize: root.titleSize
                font.bold: true
            }

            Txt {
                visible: root.subtitle.length > 0
                text: root.subtitle
                color: Theme.fg3
                font.family: Theme.fontName
                font.pixelSize: 11
            }
        }
    }
}
