import QtQuick
import "../../components"
import "../../services"

// Modal confirm dialog for destructive settings actions (restore
// defaults, clear clipboard history). Mirrors the rofi confirm prompts:
// Cancel aborts, the confirm action fires once. Fades in over the panel.
Rectangle {
    id: root
    anchors.fill: parent
    radius: 16
    visible: false
    z: 200
    color: "#d6151719"

    property string title: "Confirm"
    property string message: ""
    property string confirmText: "Proceed"
    signal confirmed()

    Behavior on opacity { NumberAnimation { duration: 140 } }
    opacity: 1

    function ask(t, m, c) {
        root.title = t
        root.message = m
        root.confirmText = c || "Proceed"
        root.visible = true
        root.opacity = 1
    }

    // Clicking the backdrop cancels.
    MouseArea {
        anchors.fill: parent
        onClicked: root.visible = false
    }

    Rectangle {
        width: 400
        height: 180
        anchors.centerIn: parent
        radius: 16
        color: Theme.bg2
        border.width: 1
        border.color: Theme.bg3

        Column {
            anchors.fill: parent
            anchors.margins: 18
            spacing: 12

            Txt {
                text: root.title
                color: Theme.fg
                font.family: Theme.fontName
                font.pixelSize: 14
                font.bold: true
            }

            Txt {
                width: parent.width
                height: 54
                wrapMode: Text.WordWrap
                text: root.message
                color: Theme.fg3
                font.family: Theme.fontName
                font.pixelSize: 12
            }

            Row {
                width: parent.width
                layoutDirection: Qt.RightToLeft
                spacing: 10

                Rectangle {
                    width: 116
                    height: 36
                    radius: 10
                    color: Theme.acc

                    Behavior on color { ColorAnimation { duration: 120 } }

                    Txt {
                        anchors.centerIn: parent
                        text: root.confirmText
                        color: Theme.sfg
                        font.family: Theme.fontName
                        font.pixelSize: 12
                        font.bold: true
                    }

                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            root.visible = false
                            root.confirmed()
                        }
                    }
                }

                Rectangle {
                    width: 90
                    height: 36
                    radius: 10
                    color: "transparent"
                    border.width: 1
                    border.color: Theme.fg3

                    Txt {
                        anchors.centerIn: parent
                        text: "Cancel"
                        color: Theme.fg
                        font.family: Theme.fontName
                        font.pixelSize: 12
                    }

                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.visible = false
                    }
                }
            }
        }
    }
}
