import QtQuick
import "."
import "../services"

// Apple-style Control Center module with optional › expand (StatusIcon chevron).
Item {
    id: root

    property string iconType: "wifi"
    property string title: ""
    property string subtitle: ""
    property bool active: false
    property bool muted: false
    property bool enabled: true
    property bool expandable: false
    property real iconSize: subtitle.length > 0 ? 22 : 20
    property color activeColor: Theme.acc
    property color inactiveFill: Theme.bg3
    property real radius: 16

    signal clicked()
    signal expandClicked()

    implicitWidth: 118
    implicitHeight: subtitle.length > 0 ? 72 : 64

    Rectangle {
        id: bg
        anchors.fill: parent
        radius: root.radius
        color: root.active ? root.activeColor : root.inactiveFill
        border.width: root.active ? 0 : 1
        border.color: Theme.border
        opacity: root.enabled ? 1 : 0.45

        Behavior on color {
            ColorAnimation { duration: 180; easing.type: Easing.OutCubic }
        }
    }

    scale: ma.pressed && !expandMa.containsMouse ? 0.97 : 1
    Behavior on scale {
        NumberAnimation { duration: 90; easing.type: Easing.OutCubic }
    }

    Item {
        anchors.fill: parent
        anchors.rightMargin: root.expandable ? 34 : 0

        Column {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: 14
            anchors.rightMargin: 8
            spacing: root.subtitle.length > 0 ? 6 : 4

            StatusIcon {
                type: root.iconType
                active: root.active
                muted: root.muted
                iconSize: root.iconSize
                color: root.active ? Theme.sfg : Theme.fg
            }

            Column {
                width: parent.width
                spacing: 1

                Txt {
                    width: parent.width
                    text: root.title
                    color: root.active ? Theme.sfg : Theme.fg
                    font.family: Theme.fontName
                    font.pixelSize: root.subtitle.length > 0 ? 12 : 11
                    font.bold: true
                    elide: Text.ElideRight
                }

                Txt {
                    width: parent.width
                    visible: root.subtitle.length > 0
                    text: root.subtitle
                    color: root.active ? Qt.rgba(Theme.sfg.r, Theme.sfg.g, Theme.sfg.b, 0.85) : Theme.fg3
                    font.family: Theme.fontName
                    font.pixelSize: 10
                    elide: Text.ElideRight
                }
            }
        }

        MouseArea {
            id: ma
            anchors.fill: parent
            enabled: root.enabled
            cursorShape: Qt.PointingHandCursor
            onClicked: root.clicked()
        }
    }

    Item {
        visible: root.expandable
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: 34

        Rectangle {
            width: 1
            height: parent.height * 0.42
            anchors.verticalCenter: parent.verticalCenter
            anchors.left: parent.left
            color: root.active ? Qt.rgba(Theme.sfg.r, Theme.sfg.g, Theme.sfg.b, 0.28) : Theme.border
        }

        StatusIcon {
            anchors.centerIn: parent
            type: "chevronRight"
            iconSize: 14
            color: root.active ? Theme.sfg : Theme.fg3
        }

        MouseArea {
            id: expandMa
            anchors.fill: parent
            enabled: root.enabled
            cursorShape: Qt.PointingHandCursor
            hoverEnabled: true
            onClicked: root.expandClicked()
        }
    }
}
