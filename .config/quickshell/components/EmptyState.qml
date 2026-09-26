import QtQuick
import "../services"

// Centered empty-state message (optionally with an icon). Anchor it from
// the caller: e.g. anchors.centerIn: parent.
Item {
    id: root

    property string text: ""
    property string icon: ""
    property string iconFamily: Theme.iconFont

    implicitWidth: Math.max(iconTxt.implicitWidth, label.implicitWidth)
    implicitHeight: column.height

    Column {
        id: column
        spacing: 8

        Txt {
            id: iconTxt
            anchors.horizontalCenter: parent.horizontalCenter
            visible: root.icon.length > 0
            text: root.icon
            font.family: root.iconFamily
            font.pixelSize: 22
            color: Theme.fg3
        }

        Txt {
            id: label
            anchors.horizontalCenter: parent.horizontalCenter
            text: root.text
            color: Theme.fg3
            font.family: Theme.fontName
            font.pixelSize: 13
            wrapMode: Text.WordWrap
        }
    }
}
