import QtQuick
import Quickshell
import "."
import "../services"

// Generic slider used for both volume and brightness.
// The parent binds `sliderValue` and connects to `setValue`.
Item {
    id: root
    implicitWidth: 280
    implicitHeight: 90

    property real sliderValue: 0.5
    property string iconLow: "\uf026"
    property string iconMid: "\uf027"
    property string iconHigh: "\uf028"
    // When set ("volume" / "brightness" / "mic") the glyph trio is replaced
    // by a drawn StatusIcon that tracks sliderValue.
    property string iconType: ""
    signal setValue(real value, bool live)

    property real barWidth: 220
    property real barHeight: 44
    property bool dragging: false

    function pctFromX(x) {
        var p = (x - 4) / (track.width - 8)
        return Math.max(0, Math.min(1, p))
    }

    Column {
        anchors.centerIn: parent
        spacing: 10

        Item {
            id: track
            width: root.barWidth
            height: root.barHeight
            anchors.horizontalCenter: parent.horizontalCenter

            Rectangle {
                anchors.fill: parent
                radius: height / 2
                color: "#3A3A3C"
            }

            Item {
                anchors.fill: parent
                anchors.margins: 4
                clip: false

                Rectangle {
                    id: fill
                    height: parent.height
                    width: Math.max(height, parent.width * root.sliderValue)
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    radius: height / 2
                    color: Theme.fg
                    // No Behavior while dragging: instant tracking, zero lag.
                    // Snaps smoothly only for programmatic changes (IPC calls).
                    Behavior on width {
                        enabled: !root.dragging
                        NumberAnimation { duration: 120; easing.type: Easing.OutCubic }
                    }
                }
            }

            Txt {
                visible: root.iconType.length === 0
                anchors.left: parent.left
                anchors.leftMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                text: root.sliderValue <= 0 ? root.iconLow
                      : root.sliderValue < 0.5 ? root.iconMid
                      : root.iconHigh
                font.family: Theme.fontName
                font.pixelSize: 16
                color: fill.width > (14 + 16) ? Theme.bg : Theme.fg
                z: 2
            }

            // Drawn icon mode (Apple-style): tracks the slider level, slashes
            // for muted volume / mic at 0.
            StatusIcon {
                visible: root.iconType.length > 0
                anchors.left: parent.left
                anchors.leftMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                type: root.iconType
                level: root.sliderValue
                muted: root.sliderValue <= 0
                    && (root.iconType === "volume" || root.iconType === "mic")
                iconSize: 18
                color: fill.width > (14 + 16) ? Theme.bg : Theme.fg
                z: 2
            }

            MouseArea {
                anchors.fill: parent
                onPressed: mouse => {
                    root.dragging = true
                    root.setValue(root.pctFromX(mouse.x), true)
                }
                onPositionChanged: mouse => {
                    if (pressed)
                        root.setValue(root.pctFromX(mouse.x), true)
                }
                onReleased: mouse => {
                    root.dragging = false
                    root.setValue(root.pctFromX(mouse.x), false)
                }
            }

            WheelHandler {
                onWheel: event => {
                    var delta = event.angleDelta.y > 0 ? 0.05 : -0.05
                    root.setValue(root.sliderValue + delta, false)
                }
            }
        }

        Txt {
            anchors.horizontalCenter: parent.horizontalCenter
            text: Math.round(root.sliderValue * 100) + "%"
            color: Theme.fg
            font.family: Theme.fontName
            font.pixelSize: 12
        }
    }
}