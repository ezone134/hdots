import QtQuick
import "."
import "../../components"
import "../../services"

// Alt-tab style window switcher island: lists every mapped window,
// current workspace first, Enter/click focuses the selection.
Item {
    id: root
    implicitWidth: 480
    implicitHeight: 430
    focus: true

    property var rows: []
    property int selectedIndex: 0

    // Windows ordered: current workspace first (hyprctl order), then the
    // rest. Each row carries its workspace id for the badge.
    function buildRows() {
        const all = HyprlandManager.allWindows
        const current = HyprlandManager.currentWorkspace
        const cur = []
        const other = []
        for (let i = 0; i < all.length; i++) {
            const w = all[i]
            if (w.mapped === false)
                continue
            if (w.workspaceId === current)
                cur.push(w)
            else
                other.push(w)
        }
        root.rows = cur.concat(other)
        if (root.selectedIndex >= root.rows.length)
            root.selectedIndex = Math.max(0, root.rows.length - 1)
    }

    // Shared Nerd Font glyph mapping lives in HyprlandManager (used by both
    // the window switcher and the pill's focused-window indicator).
    function switchSelected() {
        const row = root.rows[root.selectedIndex]
        if (!row)
            return
        HyprlandManager.focusWindow(row.address)
        StateController.pillReset()
    }

    function moveSelection(delta) {
        const len = root.rows.length
        if (len === 0)
            return
        root.selectedIndex = (root.selectedIndex + delta + len) % len
    }

    Component.onCompleted: {
        root.buildRows()
    }

    onVisibleChanged: {
        if (visible) {
            root.buildRows()
        }
    }

    Connections {
        target: HyprlandManager

        function onAllWindowsChanged() { root.buildRows() }
        function onCurrentWorkspaceChanged() { root.buildRows() }
    }

    OverlayFocusScope {
        focusTarget: root
    }

    Keys.onPressed: function(event) {
        if (event.key === Qt.Key_Escape) {
            StateController.pillReset()
            event.accepted = true
        } else if (event.key === Qt.Key_Up) {
            root.moveSelection(-1)
            event.accepted = true
        } else if (event.key === Qt.Key_Down) {
            root.moveSelection(1)
            event.accepted = true
        } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            root.switchSelected()
            event.accepted = true
        }
    }

    Column {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 10

        // Header
        SectionTitle {
            width: parent.width
            icon: "\uf87c"
            title: "Window Switcher"
            subtitle: root.rows.length > 0
                ? root.rows.length + " windows • Enter: switch • Esc: close"
                : "Enter: switch • Esc: close"
        }

        // Window list
        Item {
            width: parent.width
            height: parent.height - 58

            EmptyState {
                anchors.centerIn: parent
                visible: root.rows.length === 0
                text: "No windows open"
            }

            ListView {
                id: winList
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                anchors.right: parent.right
                anchors.rightMargin: 8
                clip: true
                model: root.rows
                currentIndex: root.selectedIndex
                spacing: 4

                delegate: ListItem {
                    required property int index
                    required property var modelData
                    width: winList.width
                    height: 46
                    radius: 10
                    paddingL: 10
                    paddingR: 10
                    paddingT: 5
                    paddingB: 5
                    selected: winList.currentIndex === index
                    onEntered: root.selectedIndex = index
                    onActivated: {
                        root.selectedIndex = index
                        root.switchSelected()
                    }

                    Rectangle {
                        width: 34
                        height: 34
                        radius: 8
                        color: winList.currentIndex === index ? Theme.bg : Theme.bg3
                        anchors.verticalCenter: parent.verticalCenter

                        Txt {
                            anchors.centerIn: parent
                            text: HyprlandManager.appGlyph(modelData.className)
                            font.family: Theme.iconFont
                            font.pixelSize: 17
                            color: winList.currentIndex === index ? Theme.sfg : Theme.fg
                        }
                    }

                    Column {
                        anchors.verticalCenter: parent.verticalCenter
                        width: parent.width - 120
                        spacing: 1

                        Txt {
                            width: parent.width
                            text: modelData.title
                            color: winList.currentIndex === index ? Theme.sfg : Theme.fg
                            font.family: Theme.fontName
                            font.pixelSize: 13
                            font.bold: winList.currentIndex === index
                            elide: Text.ElideRight

                            Behavior on color {
                                ColorAnimation { duration: 120 }
                            }
                        }

                        Txt {
                            width: parent.width
                            text: modelData.className + (modelData.workspaceId !== HyprlandManager.currentWorkspace ? "  •  ws " + modelData.workspaceId : "")
                            color: winList.currentIndex === index ? Theme.sfg : Theme.fg3
                            font.family: Theme.fontName
                            font.pixelSize: 11
                            opacity: winList.currentIndex === index ? 0.9 : 0.8
                            elide: Text.ElideRight
                        }
                    }

                    Row {
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 6

                        Txt {
                            visible: modelData.fullscreen === true
                            text: "\uf65e"
                            font.family: Theme.iconFont
                            font.pixelSize: 12
                            color: winList.currentIndex === index ? Theme.sfg : Theme.fg3
                        }

                        Txt {
                            visible: modelData.floating === true
                            text: "\uf7c4"
                            font.family: Theme.iconFont
                            font.pixelSize: 12
                            color: winList.currentIndex === index ? Theme.sfg : Theme.fg3
                        }
                    }
                }

                onCurrentIndexChanged: {
                    positionViewAtIndex(currentIndex, ListView.Contain)
                }
            }

            ScrollDragger {

                target: winList

            }
        }
    }
}