import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import QsHypr 1.0
import "."
import "../../components"
import "../../services"

// Keyboard category: shows the active keymap and switches layout. Reads
// `devices` once on open (no polling); switching runs
// `switchxkblayout <device> next|prev` and re-reads. Both go through QsHypr
// (in-process socket, no fork). The layout switcher lives here in
// Settings — deliberately no bar chip.
Column {
    id: root
    spacing: 12

    property string deviceName: "all"
    property string currentLayout: "…"
    property bool loading: true
    property bool ready: false

    function refreshLayout() {
        root.loading = true
        QsHypr.request("devices", true, function(text) { root.applyDevices(text) })
    }

    function switchLayout(dir) {
        QsHypr.dispatch("switchxkblayout " + root.deviceName + " " + dir, function() {
            root.refreshLayout()
        })
    }

    function applyDevices(raw) {
        root.loading = false
        const text = String(raw || "")
        if (text.length === 0) {
            root.ready = false
            return
        }
        try {
            const data = JSON.parse(text)
            const kbs = data.keyboards || []
            if (kbs.length === 0) {
                root.ready = false
                return
            }
            let kb = null
            for (let i = 0; i < kbs.length; i++) {
                if (kbs[i].main === true) {
                    kb = kbs[i]
                    break
                }
            }
            if (!kb)
                kb = kbs[0]
            root.deviceName = String(kb.name || "all")
            root.currentLayout = String(kb.active_keymap || "Unknown")
            root.ready = true
        } catch (e) {
            root.ready = false
        }
    }

    Component.onCompleted: root.refreshLayout()

    // Re-read whenever the panel becomes visible again (settings stay open
    // across category switches).
    onVisibleChanged: {
        if (visible)
            root.refreshLayout()
    }

    Txt {
        text: "Keyboard"
        color: Theme.fg
        font.family: Theme.fontName
        font.pixelSize: 13
        font.bold: true
    }

    Txt {
        width: 440
        wrapMode: Text.WordWrap
        text: "Shows the active keyboard layout and cycles through the layouts configured in hyprland.conf. No bar chip — layout switching lives here in Settings."
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 11
    }

    // Current layout card
    Rectangle {
        width: 440
        height: 88
        radius: 16
        color: Theme.bg2
        border.width: 1
        border.color: root.ready ? Theme.acc : Theme.bg3

        Row {
            anchors.fill: parent
            anchors.leftMargin: 18
            anchors.rightMargin: 18
            spacing: 16

            Rectangle {
                width: 48
                height: 48
                radius: 24
                anchors.verticalCenter: parent.verticalCenter
                color: root.ready ? Theme.acc : Theme.bg3

                Txt {
                    anchors.centerIn: parent
                    text: "\uf11c"
                    font.family: Theme.iconFont
                    font.pixelSize: 20
                    color: root.ready ? Theme.sfg : Theme.fg3
                }
            }

            Column {
                anchors.verticalCenter: parent.verticalCenter
                spacing: 3

                Txt {
                    text: root.loading ? "Reading devices…"
                        : (root.ready ? root.currentLayout : "No keyboard detected")
                    color: root.ready ? Theme.fg : Theme.fg3
                    font.family: Theme.fontName
                    font.pixelSize: 17
                    font.bold: root.ready
                }

                Txt {
                    visible: root.ready
                    text: root.deviceName
                    color: Theme.fg3
                    font.family: Theme.fontName
                    font.pixelSize: 10
                    elide: Text.ElideRight
                    width: 300
                }
            }
        }
    }

    // Previous / Next buttons
    Row {
        width: 440
        spacing: 10

        Rectangle {
            width: (parent.width - 10) / 2
            height: 40
            radius: 12
            color: prevBtn.containsMouse ? Theme.hover : Theme.bg2
            border.width: 1
            border.color: Theme.bg3

            Behavior on color {
                ColorAnimation { duration: 120 }
            }

            Txt {
                anchors.centerIn: parent
                text: "\uf053  Previous layout"
                font.family: Theme.fontName
                font.pixelSize: 12
                font.bold: true
                color: Theme.fg
            }

            MouseArea {
                id: prevBtn
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.switchLayout("prev")
            }
        }

        Rectangle {
            width: (parent.width - 10) / 2
            height: 40
            radius: 12
            color: nextBtn.containsMouse ? Theme.hover : Theme.bg2
            border.width: 1
            border.color: Theme.bg3

            Behavior on color {
                ColorAnimation { duration: 120 }
            }

            Txt {
                anchors.centerIn: parent
                text: "Next layout  \uf054"
                font.family: Theme.fontName
                font.pixelSize: 12
                font.bold: true
                color: Theme.fg
            }

            MouseArea {
                id: nextBtn
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.switchLayout("next")
            }
        }
    }

    Txt {
        width: 440
        wrapMode: Text.WordWrap
        text: "Tip: add more layouts in hyprland.conf's input section (e.g. kb_layout = us,de) — switching cycles through them in order."
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 10
    }
}
