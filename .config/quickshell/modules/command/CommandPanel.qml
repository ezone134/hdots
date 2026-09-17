pragma ComponentBehavior: Bound
import QtQuick
import Quickshell
import Quickshell.Io
import "."
import "../../components"
import "../../services"

Item {
    id: root
    implicitWidth: 560
    implicitHeight: 420
    focus: true

    property var actions: []
    property var filteredItems: []
    property string query: ""
    property int selectedIndex: 0

    // Debounce rebuilds so bursts of Hyprland/MPRIS/notification events
    // collapse into a single rebuild instead of one per event.
    property Timer rebuildDebounce: Timer {
        interval: 80
        repeat: false
        onTriggered: root.rebuildActionsNow()
    }

    function rebuildActions() {
        rebuildDebounce.restart()
    }

    function rebuildActionsNow() {
        const next = []

        for (let i = 0; i < AppManager.apps.length; i++) {
            const app = AppManager.apps[i]
            next.push({
                type: "app",
                title: app.name,
                subtitle: app.id,
                icon: app.icon || "",
                appId: app.id
            })
        }

        for (let i = 0; i < HyprlandManager.allWindows.length; i++) {
            const win = HyprlandManager.allWindows[i]
            next.push({
                type: "window",
                title: win.title,
                subtitle: win.className + " • Workspace " + win.workspaceId,
                icon: "󰖯",
                address: win.address
            })
        }

        for (let i = 0; i < HyprlandManager.workspaceIds.length; i++) {
            const workspaceId = HyprlandManager.workspaceIds[i]
            next.push({
                type: "workspace",
                title: "Workspace " + workspaceId,
                subtitle: "Switch workspace",
                icon: "󰍹",
                workspaceId: workspaceId
            })
        }

        next.push({
            type: "action",
            title: "Open App Launcher",
            subtitle: "Show applications",
            icon: "󰀻",
            actionId: "launcher"
        })

        next.push({
            type: "action",
            title: "Open Workspace Switcher",
            subtitle: "Switch windows and workspaces",
            icon: "󰍺",
            actionId: "workspace"
        })

        next.push({
            type: "action",
            title: "Open MPRIS",
            subtitle: MprisManager.available ? MprisManager.displayText : "Media controls",
            icon: MprisManager.status === "Playing" ? "󰝚" : "󰐊",
            actionId: "mpris"
        })

        next.push({
            type: "action",
            title: MprisManager.status === "Playing" ? "Pause Media" : "Play Media",
            subtitle: MprisManager.available ? MprisManager.displayText : "Toggle playback",
            icon: MprisManager.status === "Playing" ? "󰏤" : "󰐊",
            actionId: "mpris-toggle"
        })

        next.push({
            type: "action",
            title: "Next Track",
            subtitle: "MPRIS next",
            icon: "󰒭",
            actionId: "mpris-next"
        })

        next.push({
            type: "action",
            title: "Previous Track",
            subtitle: "MPRIS previous",
            icon: "󰒮",
            actionId: "mpris-prev"
        })

        next.push({
            type: "action",
            title: "Open Wallpaper Picker",
            subtitle: "Browse wallpapers",
            icon: "󰸉",
            actionId: "wallpaper"
        })

        next.push({
            type: "action",
            title: "Open Control Center",
            subtitle: "Quick settings",
            icon: "󰒓",
            actionId: "control"
        })

        next.push({
            type: "action",
            title: "Open Notification Center",
            subtitle: NotificationManager.unreadCount > 0 ? String(NotificationManager.unreadCount) + " unread" : "Browse notification history",
            icon: "󰂚",
            actionId: "notifications"
        })

        next.push({
            type: "action",
            title: NotificationManager.dndEnabled ? "Disable Do Not Disturb" : "Enable Do Not Disturb",
            subtitle: "Toggle notification popups",
            icon: NotificationManager.dndEnabled ? "󰂛" : "󰂚",
            actionId: "toggle-dnd"
        })

        next.push({
            type: "action",
            title: "Set Volume 50%",
            subtitle: "Audio quick action",
            icon: "󰕾",
            actionId: "volume-50"
        })

        next.push({
            type: "action",
            title: BrightnessManager.brightnessValue >= 0.5 ? "Dim Brightness" : "Raise Brightness",
            subtitle: "Display quick action",
            icon: "󰃟",
            actionId: "brightness-toggle"
        })

        for (let i = 0; i < WallpaperManager.wallpapers.length && i < 8; i++) {
            const path = WallpaperManager.wallpapers[i]
            const parts = String(path).split("/")
            const name = parts[parts.length - 1]
            next.push({
                type: "wallpaper",
                title: name,
                subtitle: "Apply wallpaper",
                icon: "󰸉",
                path: path
            })
        }

        actions = next
        filterItems(query)
    }

    function filterItems(text) {
        query = String(text || "")
        const q = query.trim().toLowerCase()
        if (q.length === 0) {
            filteredItems = actions
        } else {
            filteredItems = actions.filter(function(item) {
                return String(item.title || "").toLowerCase().indexOf(q) !== -1
                    || String(item.subtitle || "").toLowerCase().indexOf(q) !== -1
                    || String(item.type || "").toLowerCase().indexOf(q) !== -1
            })
        }
        selectedIndex = filteredItems.length > 0 ? 0 : -1
    }

    function moveSelection(delta) {
        const len = filteredItems.length
        if (len <= 0)
            return
        selectedIndex = (selectedIndex + delta + len) % len
    }

    function activateSelected() {
        if (selectedIndex < 0 || selectedIndex >= filteredItems.length)
            return

        const item = filteredItems[selectedIndex]
        if (!item)
            return

        if (item.type === "app") {
            AppManager.launch(item.appId)
            StateController.pillReset()
            return
        }

        if (item.type === "window") {
            HyprlandManager.focusWindow(item.address)
            StateController.pillReset()
            return
        }

        if (item.type === "workspace") {
            HyprlandManager.focusWorkspace(item.workspaceId)
            StateController.pillReset()
            return
        }

        if (item.type === "wallpaper") {
            WallpaperManager.setWallpaper(item.path)
            StateController.pillReset()
            return
        }

        if (item.actionId === "launcher") {
            StateController.launcher()
        } else if (item.actionId === "workspace") {
            StateController.workspaceSwitcher()
        } else if (item.actionId === "mpris") {
            StateController.mpris()
        } else if (item.actionId === "mpris-toggle") {
            MprisManager.playPause()
            StateController.pillReset()
        } else if (item.actionId === "mpris-next") {
            MprisManager.next()
            StateController.pillReset()
        } else if (item.actionId === "mpris-prev") {
            MprisManager.previous()
            StateController.pillReset()
        } else if (item.actionId === "wallpaper") {
            StateController.wallpaper()
        } else if (item.actionId === "control") {
            StateController.controlCenter()
        } else if (item.actionId === "notifications") {
            StateController.notificationCenter()
        } else if (item.actionId === "toggle-dnd") {
            NotificationManager.toggleDnd()
            StateController.pillReset()
        } else if (item.actionId === "volume-50") {
            VolumeManager.setVolume(0.5, false)
            StateController.pillReset()
        } else if (item.actionId === "brightness-toggle") {
            const target = BrightnessManager.brightnessValue >= 0.5 ? 0.25 : 0.75
            BrightnessManager.setBrightness(target, false)
            StateController.pillReset()
        }
    }

    function resolveIcon(item) {
        if (item.type !== "app")
            return ""
        if (!item.icon)
            return ""
        if (item.icon.indexOf("/") === 0)
            return "file://" + item.icon

        return AppManager.resolveIcon(item.icon)
    }

    Component.onCompleted: {
        AppManager.ensureLoaded()
        WallpaperManager.trigger()
        rebuildActions()
    }

    Connections {
        target: AppManager
        function onAppsChanged() { root.rebuildActions() }
    }

    Connections {
        target: HyprlandManager
        function onAllWindowsChanged() { root.rebuildActions() }
        function onWorkspaceIdsChanged() { root.rebuildActions() }
    }

    Connections {
        target: MprisManager
        function onTitleChanged() { root.rebuildActions() }
        function onArtistChanged() { root.rebuildActions() }
        function onStatusChanged() { root.rebuildActions() }
        function onAvailableChanged() { root.rebuildActions() }
    }

    Connections {
        target: WallpaperManager
        function onWallpapersChanged() { root.rebuildActions() }
    }

    Connections {
        target: NotificationManager
        function onUnreadCountChanged() { root.rebuildActions() }
        function onDndEnabledChanged() { root.rebuildActions() }
    }

    onVisibleChanged: {
        if (visible) {
            AppManager.ensureLoaded()
            rebuildActions()
        }
    }

    OverlayFocusScope {
        delay: 120
        focusTarget: searchInput
    }



    Rectangle {
        anchors.fill: parent
        radius: 24
        color: Theme.bg
    }

    Column {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 10

        Rectangle {
            width: parent.width
            height: 42
            radius: 12
            color: "transparent"

            Txt {
                anchors.left: parent.left
                anchors.leftMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                text: "Search apps, windows, workspaces, actions..."
                color: Theme.fg3
                font.family: Theme.fontName
                font.pixelSize: 14
                visible: searchInput.text.length === 0
            }

            TxtInput {
                id: searchInput
                anchors.fill: parent
                anchors.margins: 14
                color: Theme.fg
                font.family: Theme.fontName
                font.pixelSize: 14

                Keys.onUpPressed: function(event) {
                    root.moveSelection(-1)
                    event.accepted = true
                }
                Keys.onDownPressed: function(event) {
                    root.moveSelection(1)
                    event.accepted = true
                }
                Keys.onReturnPressed: function(event) {
                    root.activateSelected()
                    event.accepted = true
                }
                Keys.onEnterPressed: function(event) {
                    root.activateSelected()
                    event.accepted = true
                }
                Keys.onEscapePressed: function(event) {
                    StateController.pillReset()
                    event.accepted = true
                }

                onTextChanged: root.filterItems(text)
            }
        }

        Item {
            width: parent.width
            height: parent.height - 52

            ListView {
                id: resultList
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                anchors.right: parent.right
                anchors.rightMargin: 8
                clip: true
                model: root.filteredItems
                currentIndex: root.selectedIndex

                delegate: Item {
                    id: resultDelegate
                    required property int index
                    required property var modelData
                    width: resultList.width
                    height: 46

                    Rectangle {
                        anchors.fill: parent
                        radius: 10
                        color: resultList.currentIndex === resultDelegate.index ? Theme.acc : "transparent"
                        border.color: resultList.currentIndex === resultDelegate.index ? Theme.acc : "transparent"
                        border.width: resultList.currentIndex === resultDelegate.index ? 1 : 0

                        Row {
                            anchors.fill: parent
                            anchors.leftMargin: 10
                            anchors.rightMargin: 10
                            spacing: 10

                            Rectangle {
                                width: 24
                                height: 24
                                radius: 6
                                anchors.verticalCenter: parent.verticalCenter
                                color: resultList.currentIndex === resultDelegate.index ? Theme.bg : "transparent"

                                Image {
                                    anchors.fill: parent
                                    anchors.margins: 3
                                    source: root.resolveIcon(resultDelegate.modelData)
                                    fillMode: Image.PreserveAspectFit
                                    asynchronous: true
                                    visible: source.length > 0
                                }

                                Txt {
                                    anchors.centerIn: parent
                                    visible: !parent.children[0].visible
                                    text: resultDelegate.modelData.icon || "•"
                                    color: resultList.currentIndex === resultDelegate.index ? Theme.sfg : Theme.fg
                                    font.family: Theme.iconFont
                                    font.pixelSize: 13
                                }
                            }

                            Column {
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 1
                                width: parent.width - 70

                                Txt {
                                    text: resultDelegate.modelData.title
                                    color: resultList.currentIndex === resultDelegate.index ? Theme.sfg : Theme.fg
                                    font.family: Theme.fontName
                                    font.pixelSize: 13
                                    font.bold: resultList.currentIndex === resultDelegate.index
                                    width: parent.width
                                    elide: Text.ElideRight
                                }

                                Txt {
                                    text: resultDelegate.modelData.subtitle || ""
                                    color: resultList.currentIndex === resultDelegate.index ? Theme.sfg : Theme.fg3
                                    font.family: Theme.fontName
                                    font.pixelSize: 11
                                    width: parent.width
                                    elide: Text.ElideRight
                                }
                            }
                        }

                        MouseArea {
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onEntered: root.selectedIndex = resultDelegate.index
                            onPositionChanged: root.selectedIndex = resultDelegate.index
                            onClicked: {
                                root.selectedIndex = resultDelegate.index
                                root.activateSelected()
                            }
                        }
                    }
                }

                onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
            }

            ScrollDragger {

                target: resultList

            }
        }
    }
}