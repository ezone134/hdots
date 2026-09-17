import QtQuick
import "../../components"
import "../../services"

// Wallpaper category: opens the wallpaper picker island or rescans the
// thumbnail cache (both quickshell-native, no rofi involved).
Column {
    spacing: 14

    Txt {
        width: 420
        wrapMode: Text.WordWrap
        text: "Wallpapers are scanned from your thumbnail cache. Pick one from the picker, or rescan the cache if new images were added."
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 12
    }

    Row {
        spacing: 10

        Rectangle {
            width: 210
            height: 44
            radius: 12
            color: Theme.acc

            Txt {
                anchors.centerIn: parent
                text: "Open Wallpaper Picker"
                color: Theme.sfg
                font.family: Theme.fontName
                font.pixelSize: 13
                font.bold: true
            }

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: StateController.wallpaper()
            }
        }

        Rectangle {
            width: 150
            height: 44
            radius: 12
            color: Theme.bg2
            border.width: 1
            border.color: Theme.bg3

            Txt {
                anchors.centerIn: parent
                text: "Rescan Cache"
                color: Theme.fg
                font.family: Theme.fontName
                font.pixelSize: 13
                font.bold: true
            }

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: WallpaperManager.trigger()
            }
        }
    }
}
