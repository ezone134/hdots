import QtQuick
import "../services"

// Scroll track + accent thumb for a Flickable/ListView. Drop-in for the
// scrollbar block that was duplicated across the list panels: instantiate
// as a child of the list's container and point it at the view.
Item {
    id: root

    property Flickable target: null
    property int thumbWidth: 8
    property color thumbColor: Theme.acc
    property int minThumbHeight: 36

    readonly property bool canScroll: target ? target.contentHeight > target.height : false
    readonly property real maxScroll: Math.max(0, (target ? target.contentHeight : 0) - (target ? target.height : 0))

    anchors.top: parent.top
    anchors.bottom: parent.bottom
    anchors.right: parent.right
    width: thumbWidth

    Rectangle {
        id: track
        anchors.fill: parent
        color: "transparent"
        visible: root.canScroll

        Rectangle {
            id: thumb
            width: parent.width
            radius: width / 2
            color: root.thumbColor
            y: root.maxScroll > 0 ? (track.height - height) * (target.contentY / root.maxScroll) : 0
            height: {
                if (target.contentHeight <= 0)
                    return track.height
                return Math.max(root.minThumbHeight, track.height * (target.height / target.contentHeight))
            }

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.OpenHandCursor
                property real pressOffset: 0

                onPressed: mouse => {
                    pressOffset = mouse.y
                }

                onPositionChanged: mouse => {
                    if (!root.canScroll)
                        return
                    const range = track.height - thumb.height
                    const thumbY = Math.max(0, Math.min(range, thumb.y + mouse.y - pressOffset))
                    target.contentY = (thumbY / range) * root.maxScroll
                }
            }
        }
    }
}
