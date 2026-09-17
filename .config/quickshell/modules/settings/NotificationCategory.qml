import QtQuick
import "../../components"
import "../../services"

// Notifications category: DND, timeout, max toasts, toast width, corner,
// plus category filter toggles (scripts always notify-send; QS filters here).
Column {
    id: root
    spacing: 16

    function timeoutToValue(ms) { return Math.max(0, Math.min(1, (ms - 1000) / 14000)) }
    function valueToTimeout(v) { return Math.round(1000 + v * 14000) }
    function maxToastsToValue(v) { return Math.max(0, Math.min(1, (v - 1) / 9)) }
    function valueToMaxToasts(v) { return Math.round(1 + v * 9) }
    function toastWidthToValue(v) { return Math.max(0, Math.min(1, (v - 240) / 180)) }
    function valueToToastWidth(v) { return Math.round(240 + v * 180) }

    readonly property var filterRows: [
        { key: "theme", label: "Theme" },
        { key: "theme_extras", label: "Theme extras" },
        { key: "opacity", label: "Opacity" },
        { key: "shader", label: "Shader" },
        { key: "hyprsunset", label: "Eye care" },
        { key: "shadow", label: "Shadow" },
        { key: "blur", label: "Blur" },
        { key: "battery", label: "Battery" },
        { key: "clip", label: "Clipboard" },
        { key: "caps_lock", label: "Caps lock" },
        { key: "audio_pop", label: "Audio popup" },
        { key: "login_splash", label: "Login splash" },
        { key: "extractor", label: "Extractor" },
        { key: "symlink", label: "Symlink" },
        { key: "wallpaper", label: "Wallpaper" },
        { key: "kill_mode", label: "Kill mode" },
        { key: "brightness", label: "Brightness" },
        { key: "recording", label: "Recording" },
        { key: "system", label: "System" }
    ]

    function filterOn(key) {
        const f = (SettingsManager.config.notifications && SettingsManager.config.notifications.filters) || {}
        const v = f[key]
        return v === undefined ? true : (v === true || v === 1 || v === "1")
    }

    function setAllFilters(on) {
        const next = {}
        for (let i = 0; i < root.filterRows.length; i++)
            next[root.filterRows[i].key] = on
        SettingsManager.set("notifications", "filters", next)
        NotificationManager.applyFilters(next)
    }

    Txt {
        text: "Do Not Disturb"
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 11
    }

    Rectangle {
        width: 150
        height: 40
        radius: 20
        color: NotificationManager.dndEnabled ? Theme.warning : Theme.bg2
        border.width: 1
        border.color: NotificationManager.dndEnabled ? Theme.warning : Theme.bg3

        Behavior on color {
            ColorAnimation { duration: 150 }
        }

        Txt {
            anchors.centerIn: parent
            text: NotificationManager.dndEnabled ? "ON" : "OFF"
            color: NotificationManager.dndEnabled ? Theme.bg : Theme.fg
            font.family: Theme.fontName
            font.pixelSize: 13
            font.bold: true
        }

        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: {
                NotificationManager.toggleDnd()
                SettingsManager.set("notifications", "dnd", NotificationManager.dndEnabled)
            }
        }
    }

    Txt {
        text: "Notification timeout: " + Math.round((SettingsManager.config.notifications.timeoutMs || 5000) / 1000) + "s"
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 11
    }

    Item {
        id: timeoutTrack
        width: 400
        height: 24

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
                id: timeoutFill
                height: parent.height
                width: Math.max(height, parent.width * root.timeoutToValue(SettingsManager.config.notifications.timeoutMs || 5000))
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                radius: height / 2
                color: Theme.acc
            }
        }

        MouseArea {
            anchors.fill: parent
            onPressed: mouse => {
                SettingsManager.set("notifications", "timeoutMs", root.valueToTimeout(timeoutTrack.pctFromX(mouse.x)))
            }
            onPositionChanged: mouse => {
                if (pressed)
                    SettingsManager.set("notifications", "timeoutMs", root.valueToTimeout(timeoutTrack.pctFromX(mouse.x)))
            }
        }

        function pctFromX(x) {
            var p = (x - 3) / (width - 6)
            return Math.max(0, Math.min(1, p))
        }
    }

    Txt {
        text: "Max toasts visible: " + (SettingsManager.config.notifications.maxVisible || 4)
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 11
    }

    Item {
        id: maxToastTrack
        width: 400
        height: 24

        Rectangle {
            anchors.fill: parent
            radius: height / 2
            color: Theme.bg2
        }

        Item {
            anchors.fill: parent
            anchors.margins: 3

            Rectangle {
                id: maxToastFill
                height: parent.height
                width: Math.max(height, parent.width * root.maxToastsToValue(SettingsManager.config.notifications.maxVisible || 4))
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                radius: height / 2
                color: Theme.acc
            }
        }

        MouseArea {
            anchors.fill: parent
            onPressed: mouse => {
                SettingsManager.set("notifications", "maxVisible", root.valueToMaxToasts(maxToastTrack.pctFromX(mouse.x)))
            }
            onPositionChanged: mouse => {
                if (pressed)
                    SettingsManager.set("notifications", "maxVisible", root.valueToMaxToasts(maxToastTrack.pctFromX(mouse.x)))
            }
        }

        function pctFromX(x) {
            var p = (x - 3) / (width - 6)
            return Math.max(0, Math.min(1, p))
        }
    }

    Txt {
        text: "Toast width: " + (SettingsManager.config.notifications.width || 300) + "px"
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 11
    }

    Item {
        id: toastWidthTrack
        width: 400
        height: 24

        Rectangle {
            anchors.fill: parent
            radius: height / 2
            color: Theme.bg2
        }

        Item {
            anchors.fill: parent
            anchors.margins: 3

            Rectangle {
                id: toastWidthFill
                height: parent.height
                width: Math.max(height, parent.width * root.toastWidthToValue(SettingsManager.config.notifications.width || 300))
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                radius: height / 2
                color: Theme.acc
            }
        }

        MouseArea {
            anchors.fill: parent
            onPressed: mouse => {
                SettingsManager.set("notifications", "width", root.valueToToastWidth(toastWidthTrack.pctFromX(mouse.x)))
            }
            onPositionChanged: mouse => {
                if (pressed)
                    SettingsManager.set("notifications", "width", root.valueToToastWidth(toastWidthTrack.pctFromX(mouse.x)))
            }
        }

        function pctFromX(x) {
            var p = (x - 3) / (width - 6)
            return Math.max(0, Math.min(1, p))
        }
    }

    Txt {
        text: "Toast corner"
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 11
    }

    Row {
        spacing: 10

        property var corners: [
            { anchor: "top-left", glyph: "\u2196", label: "TL" },
            { anchor: "top-right", glyph: "\u2197", label: "TR" },
            { anchor: "bottom-left", glyph: "\u2199", label: "BL" },
            { anchor: "bottom-right", glyph: "\u2198", label: "BR" }
        ]

        Repeater {
            model: parent.corners

            delegate: Rectangle {
                required property var modelData
                id: cornerBtn

                property bool active: (SettingsManager.config.notifications.anchor || "top-right") === modelData.anchor

                width: 86
                height: 40
                radius: 12
                color: cornerBtn.active ? Theme.acc : Theme.bg2
                border.width: 1
                border.color: cornerBtn.active ? Theme.acc : Theme.bg3

                Behavior on color {
                    ColorAnimation { duration: 120 }
                }

                Txt {
                    anchors.centerIn: parent
                    text: modelData.glyph + "  " + modelData.label
                    color: cornerBtn.active ? Theme.sfg : Theme.fg
                    font.family: Theme.fontName
                    font.pixelSize: 13
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: SettingsManager.set("notifications", "anchor", modelData.anchor)
                }
            }
        }
    }

    Rectangle {
        width: parent.width > 0 ? Math.min(parent.width, 520) : 520
        height: 1
        color: Theme.bg3
    }

    Txt {
        text: "Category filters"
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 11
    }

    Txt {
        width: 520
        wrapMode: Text.WordWrap
        text: "Scripts always send notify-send. Off here drops that category before toast/history."
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 11
        opacity: 0.85
    }

    Row {
        spacing: 8

        Rectangle {
            width: 104
            height: 32
            radius: 10
            color: Theme.acc
            Txt {
                anchors.centerIn: parent
                text: "Enable all"
                color: Theme.sfg
                font.family: Theme.fontName
                font.pixelSize: 12
                font.bold: true
            }
            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: root.setAllFilters(true)
            }
        }

        Rectangle {
            width: 104
            height: 32
            radius: 10
            color: Theme.bg2
            border.width: 1
            border.color: Theme.bg3
            Txt {
                anchors.centerIn: parent
                text: "Disable all"
                color: Theme.fg
                font.family: Theme.fontName
                font.pixelSize: 12
                font.bold: true
            }
            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: root.setAllFilters(false)
            }
        }
    }

    Column {
        spacing: 6
        width: 520

        Repeater {
            model: root.filterRows

            delegate: Rectangle {
                required property var modelData
                width: parent.width
                height: 36
                radius: 10
                color: Theme.bg2
                border.width: 1
                border.color: Theme.bg3

                property bool on: root.filterOn(modelData.key)

                Txt {
                    anchors.left: parent.left
                    anchors.leftMargin: 12
                    anchors.verticalCenter: parent.verticalCenter
                    text: modelData.label
                    color: Theme.fg
                    font.family: Theme.fontName
                    font.pixelSize: 13
                }

                Rectangle {
                    anchors.right: parent.right
                    anchors.rightMargin: 10
                    anchors.verticalCenter: parent.verticalCenter
                    width: 52
                    height: 26
                    radius: 13
                    color: parent.on ? Theme.acc : Theme.bg3

                    Txt {
                        anchors.centerIn: parent
                        text: parent.parent.on ? "ON" : "OFF"
                        color: parent.parent.on ? Theme.sfg : Theme.fg3
                        font.family: Theme.fontName
                        font.pixelSize: 11
                        font.bold: true
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            const nextOn = !parent.parent.on
                            NotificationManager.setFilter(modelData.key, nextOn)
                        }
                    }
                }
            }
        }
    }
}
