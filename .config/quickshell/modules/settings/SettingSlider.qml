import QtQuick
import "../../components"
import "../../services"

// Slider row for a SettingSection: label + formatted value on top, a
// draggable track below (same shape as the notifications/shell tracks in
// SettingsPanel). While dragging only a local preview moves; the engine
// write + apply script fire once on release so a drag is a single file
// write instead of a process per pixel. Wheel nudges by one step.
Item {
    id: root

    property var row: null
    property var engine: null

    implicitHeight: 50
    width: parent ? parent.width : 0

    property bool _dragging: false
    property real _dragValue: 0

    readonly property real _min: root.row ? Number(root.row.min) : 0
    readonly property real _max: root.row ? Number(root.row.max) : 1
    readonly property real _step: root.row ? Math.max(0.0001, Number(root.row.step)) : 1

    // Guard NaN before the engine's first refresh fills row.value.
    readonly property real _raw: parseFloat(root.row ? root.row.value : 0)
    readonly property real _value: root._dragging ? root._dragValue : (isNaN(root._raw) ? root._min : root._raw)
    readonly property real _pct: isNaN(root._value) ? 0 : (root._value - root._min) / Math.max(0.0001, root._max - root._min)

    function _clampStep(v) {
        let n = Math.round((v - root._min) / root._step) * root._step + root._min
        n = Math.max(root._min, Math.min(root._max, n))
        return Math.round(n * 1000) / 1000
    }

    function posToValue(p) {
        return root._clampStep(root._min + p * (root._max - root._min))
    }

    function fmt(v) {
        return root._step < 1 ? Number(v).toFixed(1) : String(Math.round(v))
    }

    function pctFromX(x, w) {
        return Math.max(0, Math.min(1, (x - 3) / Math.max(1, w - 6)))
    }

    opacity: root.row && root.row.enabled ? 1 : 0.45
    Behavior on opacity { NumberAnimation { duration: 120 } }

    Column {
        anchors.fill: parent
        spacing: 5

        Row {
            width: parent.width

            Txt {
                width: parent.width - valueTxt.implicitWidth - 12
                text: root.row ? root.row.label : ""
                color: Theme.fg
                font.family: Theme.fontName
                font.pixelSize: 13
                font.bold: true
                elide: Text.ElideRight
            }

            Txt {
                id: valueTxt
                anchors.verticalCenter: parent.verticalCenter
                text: root._dragging ? root.fmt(root._dragValue) : (root.row ? root.row.display : "")
                color: Theme.acc
                font.family: Theme.fontName
                font.pixelSize: 12
                font.bold: true
            }
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
                enabled: root.row ? root.row.enabled : false
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
                    if (root.engine && root.row)
                        root.engine.setValue(root.row.key, v)
                }
                onCanceled: root._dragging = false
            }

            WheelHandler {
                enabled: root.row ? root.row.enabled : false
                onWheel: event => {
                    const dir = event.angleDelta.y > 0 ? 1 : -1
                    if (root.engine && root.row)
                        root.engine.setValue(root.row.key, root._clampStep(root._value + dir * root._step))
                }
            }
        }
    }
}
