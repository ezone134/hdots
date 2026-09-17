import QtQuick
import "."
import "../../components"
import "../../services"

// Popups: channel/mode switcher + link into Notifications category filters.
// Category mute lives in $states/settings.json notifications.filters (QS).
Column {
    id: root
    anchors.fill: parent
    spacing: 10

    ChannelModeSwitcher {
        id: chSwitcher
        width: parent.width
    }

    Row {
        width: parent.width
        spacing: 8

        Rectangle {
            width: 168
            height: 36
            radius: 10
            color: Theme.bg2
            border.width: 1
            border.color: Theme.bg3

            Txt {
                anchors.centerIn: parent
                text: "\uf0f3  Notification Center"
                color: Theme.fg
                font.family: Theme.fontName
                font.pixelSize: 12
                font.bold: true
            }

            MouseArea {
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: StateController.notificationCenter()
            }
        }

        Rectangle {
            width: 200
            height: 36
            radius: 10
            color: Theme.acc

            Txt {
                anchors.centerIn: parent
                text: "Open filter toggles"
                color: Theme.sfg
                font.family: Theme.fontName
                font.pixelSize: 12
                font.bold: true
            }

            MouseArea {
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: StateController.settingsCategoryOpen("notifications")
            }
        }
    }

    Rectangle {
        width: parent.width
        height: 1
        color: Theme.bg3
    }

    Txt {
        width: parent.width
        wrapMode: Text.WordWrap
        text: "Popup category filters moved to Settings → Notifications. Scripts always notify-send; Quickshell drops muted categories."
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 12
    }

    SettingSection {
        id: section
        width: parent.width
        height: Math.max(120, parent.height - chSwitcher.height - 90)
        sections: ["notif"]
    }
}
