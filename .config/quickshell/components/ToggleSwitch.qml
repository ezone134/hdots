import QtQuick
import "."
import "../services"

// Small iOS-style switch used in the settings panel. Sits inline next to a
// label; emits toggled(value) on click, parent persists the value via
// SettingsManager.set(). No singleton dependencies, safe to instantiate.
Item {
    id: root

    property bool checked: false
    signal toggled(bool value)

    implicitWidth: 46
    implicitHeight: 24

    Rectangle {
        id: track
        anchors.fill: parent
        radius: height / 2
        color: root.checked ? Theme.acc : Theme.bg2
        border.width: 1
        border.color: root.checked ? Theme.acc : Theme.bg3

        Behavior on color {
            ColorAnimation { duration: 150 }
        }

        Behavior on border.color {
            ColorAnimation { duration: 150 }
        }
    }

    Rectangle {
        id: knob
        width: 18
        height: 18
        radius: 9
        color: root.checked ? Theme.sfg : Theme.fg2
        anchors.verticalCenter: parent.verticalCenter
        anchors.left: parent.left
        anchors.leftMargin: root.checked ? parent.width - width - 3 : 3

        Behavior on anchors.leftMargin {
            NumberAnimation {
                duration: 150
                easing.type: Easing.OutCubic
            }
        }

        Behavior on color {
            ColorAnimation { duration: 150 }
        }
    }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: root.toggled(!root.checked)
    }
}