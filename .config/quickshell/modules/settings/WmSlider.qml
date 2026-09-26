import QtQuick
import "../../components"
import "../../services"

// Slider row for WmCategory: label + formatted value on top, a draggable
// track below. Only a local preview moves while dragging; the commit fires
// once on release so a drag is a single state write + restore instead of a
// process per pixel. Wheel nudges by one step.
Item {
    id: root

    property string label: ""
    property string hint: ""
    property real min: 0
    property real max: 1
    property real step: 1
    property real value: root.min
    signal changed(real value)

    implicitHeight: 54
    width: parent ? parent.width : 0

    property bool _dragging: false
    property real _dragValue: 0

    readonly property real _value: root._dragging ? root._dragValue : root.value
    readonly property real _pct: isNaN(root._value) ? 0 : (root._value - root.min) / Math.max(0.0001, root.max - root.min)

    function clampStep(v) {
        let n = Math.round((v - root.min) / root.step) * root.step + root.min
        n = Math.max(root.min, Math.min(root.max, n))
        return Math.round(n * 1000) / 1000
    }

    function posToValue(p) {
        return root.clampStep(root.min + p * (root.max - root.min))
    }

    function fmt(v) {
        return root.step < 1 ? Number(v).toFixed(1) : String(Math.round(v))
    }

    function pctFromX(x, w) {
        return Math.max(0, Math.min(1, (x - 3) / Math.max(1, w - 6)))
    }

    Column {
        anchors.fill: parent
        spacing: 5

        Row {
            width: parent.width

            Txt {
                width: parent.width - valueTxt.implicitWidth - 12
                text: root.label
                color: Theme.fg
                font.family: Theme.fontName
                font.pixelSize: 13
                font.bold: true
                elide: Text.ElideRight
            }

            Txt {
                id: valueTxt
                anchors.verticalCenter: parent.verticalCenter
                text: root._dragging ? root.fmt(root._dragValue) : root.fmt(root.value)
                color: Theme.acc
                font.family: Theme.fontName
                font.pixelSize: 12
                font.bold: true
            }
        }

        Txt {
            visible: root.hint.length > 0
            width: parent.width
            wrapMode: Text.WordWrap
            text: root.hint
            color: Theme.fg3
            font.family: Theme.fontName
            font.pixelSize: 11
        }

        Item {
            id: track
            width: parent.width
            height: 20

            Rectangle {
                anchors.fill: parent
                radius: height / 2
                color: Theme.bg2
            }

            Item {
                anchors.fill: parent
                anchors.margins: 3
                clip: false

                Rectangle {
                    height: parent.height
                    width: Math.max(height, parent.width * root._pct)
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    radius: height / 2
                    color: Theme.acc
                }
            }

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor

                onPressed: mouse => {
                    root._dragging = true
                    root._dragValue = root.posToValue(root.pctFromX(mouse.x, track.width))
                }
                onPositionChanged: mouse => {
                    if (pressed)
                        root._dragValue = root.posToValue(root.pctFromX(mouse.x, track.width))
                }
                onReleased: mouse => {
                    const v = root.posToValue(root.pctFromX(mouse.x, track.width))
                    root._dragging = false
                    root.changed(v)
                }
                onCanceled: root._dragging = false
            }

            WheelHandler {
                onWheel: event => {
                    const dir = event.angleDelta.y > 0 ? 1 : -1
                    root.changed(root.clampStep(root._value + dir * root.step))
                }
            }
        }
    }
}
