import QtQuick
import "../services"

// Selectable list row: accent fill + border when selected, theme text
// otherwise, hover emits entered/exited, click emits activated. Content
// is any children (default property). Selection chrome matches the
// pattern used across every list panel.
Item {
    id: root

    default property alias content: contentRow.data

    property bool selected: false
    property color fill: Theme.acc
    property color borderColor: Theme.acc
    property int radius: 14
    property int padding: 10
    property int paddingL: padding
    property int paddingR: padding
    property int paddingT: padding
    property int paddingB: padding
    property bool showBorder: true

    signal entered()
    signal exited()
    signal activated()

    readonly property bool hovered: hoverArea.containsMouse

    Rectangle {
        anchors.fill: parent
        radius: root.radius
        color: root.selected ? root.fill : "transparent"
        border.width: root.selected && root.showBorder ? 1 : 0
        border.color: root.borderColor

        Behavior on color {
            ColorAnimation { duration: 120 }
        }
    }

    Row {
        id: contentRow
        anchors.fill: parent
        anchors.leftMargin: root.paddingL
        anchors.rightMargin: root.paddingR
        anchors.topMargin: root.paddingT
        anchors.bottomMargin: root.paddingB
        spacing: 12
    }

    MouseArea {
        id: hoverArea
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onEntered: root.entered()
        onExited: root.exited()
        onClicked: root.activated()
    }
}
