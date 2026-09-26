import QtQuick
import "."

// Transition placeholder: holds the container at the previous size
// while the spring animation morphs to the new component's size.
Item {
    id: root
    clip: true
    implicitWidth: 0
    implicitHeight: 0

    Rectangle {
        anchors.fill: parent
        radius: Math.min(root.height / 2, 34)
        color: "transparent"
    }
}