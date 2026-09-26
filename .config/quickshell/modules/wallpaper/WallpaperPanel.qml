import QtQuick
import Quickshell
import "."
import "../../components"
import "../../services"

Item {
    id: root

    implicitWidth: 720
    implicitHeight: 420

    focus: true

    Component.onCompleted: WallpaperManager.trigger()

    Keys.onPressed: event => {
        if (event.key === Qt.Key_Escape) {
            StateController.pillReset()
            event.accepted = true
        }
    }

    Column {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 12

        Txt {
            anchors.horizontalCenter: parent.horizontalCenter
            text: "Wallpapers"
            color: Theme.fg
            font.family: Theme.fontName
            font.pixelSize: 16
        }

        // Fade the whole grid in with a short delay instead of items
        // sliding down as they load.
        Item {
            width: parent.width
            height: parent.height - 28
            opacity: 0
            scale: 0.98

            Behavior on opacity {
                NumberAnimation { duration: 200; easing.type: Easing.OutCubic }
            }
            Behavior on scale {
                NumberAnimation { duration: 200; easing.type: Easing.OutCubic }
            }

            Timer {
                id: fadeInTimer
                interval: 80
                repeat: false
                running: true
                onTriggered: {
                    parent.opacity = 1
                    parent.scale = 1
                }
            }

            GridView {
                id: wallpaperGrid
                anchors.fill: parent
                cellWidth: 168
                cellHeight: 108
                clip: true

                // Calculate offset to center the grid items if they take up less space than the container
                property int cols: Math.max(1, Math.floor(width / cellWidth))
                property int rows: Math.ceil(count / cols)
                property real contentW: cols * cellWidth
                property real contentH: rows * cellHeight
                x: contentW < width ? (width - contentW) / 2 : 0

                model: WallpaperManager.wallpapers

                delegate: Item {
                    width: wallpaperGrid.cellWidth
                    height: wallpaperGrid.cellHeight

                    Rectangle {
                        anchors.centerIn: parent
                        width: 152
                        height: 92
                        radius: 10
                        color: Theme.bg2
                        clip: true

                        Image {
                            anchors.fill: parent
                            source: "file://" + modelData
                            fillMode: Image.PreserveAspectCrop
                            asynchronous: true
                            smooth: true
                        }

                        Rectangle {
                            anchors.fill: parent
                            radius: parent.radius
                            color: "transparent"
                            border.color: hoverArea.containsMouse ? Theme.fg : "transparent"
                            border.width: 2
                        }

                        MouseArea {
                            id: hoverArea
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                WallpaperManager.setWallpaper(modelData)
                                StateController.pillReset()
                            }
                        }
                    }
                }
            }
        }
    }
}