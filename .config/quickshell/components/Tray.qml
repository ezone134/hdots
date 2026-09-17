pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import Quickshell.Wayland
import Quickshell.Services.SystemTray
import "../services"

// System tray (StatusNotifierItem): a chip grid for the pill's status row.
// Left-click activates (menu when onlyMenu), middle-click secondary action,
// right-click opens the native menu in its own overlay window, wheel scrolls.
// Menu window grabs exclusive keyboard focus and closes on Esc / click-away.
Item {
    id: root

    property int chipSize: 24
    property int chipSpacing: 3

    readonly property bool hasItems: SystemTray.items.values.length > 0
    visible: hasItems
    implicitWidth: visible ? row.implicitWidth : 0
    implicitHeight: root.chipSize

    // Nerd Font fallback when a StatusNotifier item doesn't resolve to a
    // theme icon (same glyph map pattern as WindowPanel.glyphFor).
    function glyphFor(item) {
        const text = String((item.id || "") + " " + (item.title || "") + " " + (item.tooltipTitle || "")).toLowerCase()
        const glyphs = {
            "spotify": "\uf1bc", "discord": "\uf6a7", "vesktop": "\uf6a7",
            "telegram": "\uf1d8", "slack": "\uf198", "steam": "\uf1b6",
            "signal": "\uf5b7", "whatsapp": "\uf232", "element": "\uf8d3",
            "firefox": "\uf269", "chrome": "\uf268", "chromium": "\uf268",
            "brave": "\uf268", "vivaldi": "\uf268", "code": "\uf121",
            "vscodium": "\uf121", "cursor": "\uf121", "zed": "\uf121",
            "mail": "\uf0e0", "thunderbird": "\uf0e0", "claws-mail": "\uf0e0",
            "syncthing": "\uf1e0", "bluetooth": "\uf294", "blueman": "\uf294",
            "network": "\uf1eb", "wifi": "\uf1eb", "nm-applet": "\uf1eb",
            "volume": "\uf028", "pipewire": "\uf028", "pavucontrol": "\uf028",
            "battery": "\uf240", "power": "\uf011", "clipboard": "\uf0ea",
            "calendar": "\uf133", "notes": "\uf249", "vpn": "\uf023",
            "proton": "\uf023", "tailscale": "\uf231", "github": "\uf09b",
            "gitkraken": "\uf1d3", "nextcloud": "\uf0c2", "dropbox": "\uf16b",
            "docker": "\uf308", "obsidian": "\uf0c4", "obs": "\uf16d",
            "zoom": "\uf2d0", "qbittorrent": "\uf0b1", "transmission": "\uf0b1",
            "mpv": "\uf008", "vlc": "\uf1c7", "media": "\uf008"
        }
        for (const key in glyphs) {
            if (text.indexOf(key) !== -1)
                return glyphs[key]
        }
        return ""
    }

    function showMenu(item, anchorItem) {
        if (!item.hasMenu)
            return
        menuOpener.menu = item.menu
        const p = anchorItem.mapToItem(null, anchorItem.width / 2, anchorItem.height)
        menu.anchorX = p.x
        menu.anchorY = p.y
        menu.open = true
    }

    QsMenuOpener {
        id: menuOpener
    }

    Row {
        id: row
        anchors.fill: parent
        spacing: root.chipSpacing

        Repeater {
            model: SystemTray.items

            delegate: Item {
                id: slot
                required property var modelData

                width: root.chipSize
                height: root.chipSize

                readonly property bool hovered: slotArea.containsMouse

                Rectangle {
                    anchors.fill: parent
                    radius: root.chipSize / 2
                    color: slot.hovered ? Theme.hover : "transparent"

                    Behavior on color {
                        ColorAnimation { duration: 120 }
                    }
                }

                Image {
                    id: trayIcon
                    anchors.centerIn: parent
                    width: 16
                    height: 16
                    sourceSize.width: 32
                    sourceSize.height: 32
                    source: slot.modelData.icon
                    fillMode: Image.PreserveAspectFit
                    smooth: true
                    asynchronous: true
                    cache: true
                    visible: source !== ""
                }

                Txt {
                    anchors.centerIn: parent
                    visible: trayIcon.source === "" || trayIcon.status === Image.Error
                    text: root.glyphFor(slot.modelData)
                    font.family: Theme.iconFont
                    font.pixelSize: 14
                    color: slot.hovered ? Theme.acc : Theme.fg

                    Behavior on color {
                        ColorAnimation { duration: 120 }
                    }
                }

                MouseArea {
                    id: slotArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
                    onClicked: (mouse) => {
                        if (mouse.button === Qt.MiddleButton) {
                            slot.modelData.secondaryActivate()
                        } else if (mouse.button === Qt.RightButton) {
                            root.showMenu(slot.modelData, slot)
                        } else if (slot.modelData.onlyMenu) {
                            root.showMenu(slot.modelData, slot)
                        } else {
                            slot.modelData.activate()
                        }
                    }
                    onWheel: (wheel) => {
                        slot.modelData.scroll(wheel.angleDelta.y, false)
                    }
                }

                // Tooltip below the chip (the pill hugs the top edge, so
                // below is always on-screen).
                Rectangle {
                    id: tip
                    visible: slot.hovered && !menu.open && tipTxt.text !== ""
                    width: tipTxt.implicitWidth + 16
                    height: 24
                    radius: 8
                    color: Theme.bg2
                    border.width: 1
                    border.color: Theme.border
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.top: parent.bottom
                    anchors.topMargin: 6
                    z: 999

                    Txt {
                        id: tipTxt
                        anchors.centerIn: parent
                        text: slot.modelData.tooltipTitle || slot.modelData.title || ""
                        color: Theme.fg
                        font.pixelSize: 11
                    }
                }
            }
        }
    }

    // One menu line: separator, or a row with optional checkbox/radio state,
    // label and a submenu chevron that rotates when expanded. Used for both
    // top-level entries and indented submenu children.
    component MenuRow: Item {
        id: mrow

        property var entryData
        property real indent: 0
        property bool expanded: false
        signal activated()

        width: parent ? parent.width : 0
        height: mrow.entryData && mrow.entryData.isSeparator ? 9 : 32

        Rectangle {
            visible: mrow.entryData && mrow.entryData.isSeparator
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: 8 + mrow.indent
            anchors.rightMargin: 8
            anchors.verticalCenter: parent.verticalCenter
            height: 1
            color: Theme.border
        }

        Rectangle {
            visible: mrow.entryData && !mrow.entryData.isSeparator
            anchors.fill: parent
            anchors.leftMargin: mrow.indent
            radius: 8
            color: mrowArea.containsMouse && mrow.entryData.enabled ? Theme.hover : "transparent"

            Behavior on color {
                ColorAnimation { duration: 120 }
            }

            Rectangle {
                id: stateBox
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                anchors.leftMargin: 12
                readonly property bool isCheck: mrow.entryData && mrow.entryData.buttonType === QsMenuButtonType.CheckBox
                readonly property bool isRadio: mrow.entryData && mrow.entryData.buttonType === QsMenuButtonType.RadioButton
                readonly property bool present: isCheck || isRadio
                readonly property bool checked: mrow.entryData && mrow.entryData.checkState === Qt.Checked
                visible: present
                width: 11
                height: 11
                radius: isRadio ? width / 2 : 3
                color: "transparent"
                border.width: 1
                border.color: checked ? Theme.acc : Theme.border

                Rectangle {
                    anchors.centerIn: parent
                    visible: stateBox.checked
                    width: 5
                    height: 5
                    radius: stateBox.isRadio ? width / 2 : 1.5
                    color: Theme.acc
                }
            }

            Txt {
                anchors.left: stateBox.right
                anchors.leftMargin: 10
                anchors.verticalCenter: parent.verticalCenter
                anchors.right: parent.right
                anchors.rightMargin: 26
                text: mrow.entryData ? mrow.entryData.text || "" : ""
                color: !mrow.entryData.enabled ? Theme.fg3 : (mrowArea.containsMouse ? Theme.fg : Theme.fg2)
                font.pixelSize: 12
                elide: Text.ElideRight
            }

            Txt {
                anchors.right: parent.right
                anchors.rightMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                visible: mrow.entryData && mrow.entryData.hasChildren === true
                text: "\uf054"
                font.family: Theme.iconFont
                font.pixelSize: 10
                rotation: mrow.expanded ? 90 : 0
                color: mrowArea.containsMouse ? Theme.fg : Theme.fg3

                Behavior on rotation {
                    NumberAnimation { duration: 100 }
                }
            }

            MouseArea {
                id: mrowArea
                anchors.fill: parent
                hoverEnabled: true
                enabled: mrow.entryData ? mrow.entryData.enabled : false
                cursorShape: Qt.PointingHandCursor
                onClicked: mrow.activated()
            }
        }
    }

    // Native menu card in its own overlay window. Full-screen so a click
    // anywhere outside the card closes it; exclusive keyboard focus so Esc
    // reaches the card and can't be swallowed by the shell's key handler.
    PanelWindow {
        id: menu

        property bool open: false
        property real anchorX: 0
        property real anchorY: 0

        onOpenChanged: {
            if (!open)
                menuOpener.menu = null
        }

        screen: root.window ? root.window.screen : null
        visible: open
        color: "transparent"

        exclusionMode: ExclusionMode.Ignore
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
        WlrLayershell.namespace: "pill-tray"

        anchors {
            top: true
            left: true
            right: true
            bottom: true
        }

        MouseArea {
            anchors.fill: parent
            onClicked: menu.open = false
        }

        FocusScope {
            anchors.fill: parent
            focus: menu.open

            Keys.onEscapePressed: menu.open = false

            Rectangle {
                id: card
                x: Math.max(8, Math.min(menu.anchorX - card.width / 2, menu.width - card.width - 8))
                y: menu.anchorY + 6
                width: 230
                radius: 12
                color: Theme.bg2
                border.width: 1
                border.color: Theme.border
                clip: true
                layer.enabled: true
                layer.samples: 2

                property int expandedIdx: -1

                implicitHeight: col.implicitHeight + 12

                Column {
                    id: col
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: 6
                    spacing: 0

                    Repeater {
                        model: menuOpener.children ? menuOpener.children.values : []

                        delegate: Column {
                            id: entry
                            required property var modelData
                            required property int index
                            readonly property bool expanded: card.expandedIdx === index

                            width: col.width

                            MenuRow {
                                width: parent.width
                                entryData: entry.modelData
                                expanded: entry.expanded
                                onActivated: {
                                    if (entry.modelData.hasChildren) {
                                        card.expandedIdx = entry.expanded ? -1 : entry.index
                                    } else {
                                        entry.modelData.triggered()
                                        menu.open = false
                                    }
                                }
                            }

                            QsMenuOpener {
                                id: childOpener
                                menu: entry.expanded ? entry.modelData : null
                            }

                            Repeater {
                                model: childOpener.children ? childOpener.children.values : []

                                delegate: MenuRow {
                                    required property var modelData
                                    width: entry.width
                                    indent: 14
                                    entryData: modelData
                                    onActivated: {
                                        if (!modelData.hasChildren) {
                                            modelData.triggered()
                                            menu.open = false
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
