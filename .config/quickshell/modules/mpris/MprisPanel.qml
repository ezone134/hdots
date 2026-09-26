import QtQuick
import "."
import "../../components"
import "../../services"

Item {
    id: root
    implicitWidth: 380
    implicitHeight: 92
    focus: true

    Rectangle {
        anchors.fill: parent
        radius: 28
        color: Theme.bg
    }

    Rectangle {
        id: accentBubble
        width: 56
        height: 56
        radius: 28
        anchors.left: parent.left
        anchors.leftMargin: 18
        anchors.verticalCenter: parent.verticalCenter
        color: Theme.acc

        Txt {
            anchors.centerIn: parent
            text: MprisManager.status === "Playing" ? "󰎆" : "󰐊"
            font.family: Theme.iconFont
            font.pixelSize: 22
            color: Theme.sfg
        }
    }

    Column {
        anchors.left: accentBubble.right
        anchors.leftMargin: 14
        anchors.right: controlsRow.left
        anchors.rightMargin: 14
        anchors.verticalCenter: parent.verticalCenter
        spacing: 4

        Txt {
            text: MprisManager.title.length > 0 ? MprisManager.title : "No media playing"
            color: Theme.fg
            font.family: Theme.fontName
            font.pixelSize: 15
            font.bold: true
            width: parent.width
            elide: Text.ElideRight
        }

        Txt {
            text: MprisManager.artist.length > 0 ? MprisManager.artist : "Open a player"
            color: Theme.fg3
            font.family: Theme.fontName
            font.pixelSize: 12
            width: parent.width
            elide: Text.ElideRight
        }
    }

    Row {
        id: controlsRow
        anchors.right: parent.right
        anchors.rightMargin: 18
        anchors.verticalCenter: parent.verticalCenter
        spacing: 10

        Rectangle {
            width: 24
            height: 24
            radius: 12
            color: "transparent"

            Txt {
                anchors.centerIn: parent
                text: "󰒮"
                font.family: Theme.iconFont
                font.pixelSize: 12
                color: Theme.fg
            }

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: function(mouse) {
                    mouse.accepted = true
                    MprisManager.previous()
                }
            }
        }

        Rectangle {
            width: 30
            height: 30
            radius: 15
            color: Theme.acc

            Txt {
                anchors.centerIn: parent
                text: MprisManager.status === "Playing" ? "󰏤" : "󰐊"
                font.family: Theme.iconFont
                font.pixelSize: 13
                color: Theme.sfg
            }

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: function(mouse) {
                    mouse.accepted = true
                    MprisManager.playPause()
                }
            }
        }

        Rectangle {
            width: 24
            height: 24
            radius: 12
            color: "transparent"

            Txt {
                anchors.centerIn: parent
                text: "󰒭"
                font.family: Theme.iconFont
                font.pixelSize: 12
                color: Theme.fg
            }

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: function(mouse) {
                    mouse.accepted = true
                    MprisManager.next()
                }
            }
        }
    }

    Keys.onPressed: function(event) {
        if (event.key === Qt.Key_Escape) {
            StateController.pillReset()
            event.accepted = true
        }
    }
}