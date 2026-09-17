import QtQuick
import "."
import "../services"

// Capsule slider — horizontal or vertical (thin CC style).
Item {
    id: root

    property real value: 0.5
    property string iconType: "sun"
    property bool muted: false
    property bool vertical: false
    property color fillColor: Theme.fg
    property color trackColor: Theme.bg3
    property real barHeight: 42
    property real barWidth: 52
    property int iconSize: 0
    property bool dragging: false

    signal setValue(real value, bool live)

    implicitWidth: vertical ? barWidth : 200
    implicitHeight: vertical ? 168 : barHeight

    readonly property real clamped: Math.max(0, Math.min(1, root.value))
    readonly property int resolvedIconSize: root.iconSize > 0
        ? root.iconSize
        : (root.vertical ? Math.max(12, Math.min(18, Math.round(width * 0.42))) : 16)

    function pctFromPos(x, y) {
        if (root.vertical) {
            const p = 1 - (y - 3) / Math.max(1, height - 6)
            return Math.max(0, Math.min(1, p))
        }
        const p = (x - 3) / Math.max(1, width - 6)
        return Math.max(0, Math.min(1, p))
    }

    function commit(x, y, live) {
        root.setValue(pctFromPos(x, y), live)
    }

    Rectangle {
        anchors.fill: parent
        radius: root.vertical ? width / 2 : height / 2
        color: root.trackColor
        border.width: 1
        border.color: Theme.border
    }

    Item {
        anchors.fill: parent
        anchors.margins: 2
        clip: true

        Rectangle {
            visible: !root.vertical
            height: parent.height
            width: Math.max(height, parent.width * root.clamped)
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            radius: height / 2
            color: root.fillColor

            Behavior on width {
                enabled: !root.dragging
                NumberAnimation { duration: 120; easing.type: Easing.OutCubic }
            }
        }

        Rectangle {
            visible: root.vertical
            width: parent.width
            height: Math.max(width, parent.height * root.clamped)
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            radius: width / 2
            color: root.fillColor

            Behavior on height {
                enabled: !root.dragging
                NumberAnimation { duration: 120; easing.type: Easing.OutCubic }
            }
        }
    }

    StatusIcon {
        anchors.horizontalCenter: root.vertical ? parent.horizontalCenter : undefined
        anchors.bottom: root.vertical ? parent.bottom : undefined
        anchors.bottomMargin: root.vertical ? Math.max(8, Math.round(parent.width * 0.22)) : 0
        anchors.left: root.vertical ? undefined : parent.left
        anchors.leftMargin: root.vertical ? 0 : 10
        anchors.verticalCenter: root.vertical ? undefined : parent.verticalCenter
        type: root.iconType
        level: root.value
        muted: root.muted
        iconSize: root.resolvedIconSize
        color: root.vertical
               ? (root.clamped > 0.22 ? Theme.bg : Theme.fg)
               : (root.clamped > 0.18 ? Theme.bg : Theme.fg)
        z: 2
    }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onPressed: mouse => {
            root.dragging = true
            root.commit(mouse.x, mouse.y, true)
        }
        onPositionChanged: mouse => {
            if (pressed)
                root.commit(mouse.x, mouse.y, true)
        }
        onReleased: mouse => {
            root.commit(mouse.x, mouse.y, false)
            root.dragging = false
        }
    }
}
