// App launcher — original design, built on the shell's own primitives
// (Txt/TxtInput, IconButton, ScrollDragger, EmptyState) and the AppManager
// service. Search lives at the top, rows follow the accent-fill selection
// pattern shared by every list panel (CommandPanel, ClipboardPanel, ...).
import QtQuick
import Quickshell
import Quickshell.Io
import "."
import "../../components"
import "../../services"

Item {
    id: root
    implicitWidth: 580
    implicitHeight: padding * 2 + headerHeight + gap + searchHeight + gap + listAreaHeight

    readonly property int padding: 14
    readonly property int headerHeight: 24
    readonly property int searchHeight: 46
    readonly property int itemHeight: 50
    readonly property int itemSpacing: 6
    readonly property int itemRounding: 12
    readonly property int maxShown: 8
    readonly property int gap: 12
    readonly property int containerRadius: 20

    readonly property int listAreaHeight: Math.max(56, Math.min(root.maxShown, root.filteredApps.length) * (root.itemHeight + root.itemSpacing) - root.itemSpacing)

    property var filteredApps: []
    property string searchText: ""
    property int selectedIndex: 0

    focus: true

    // Grab keyboard focus when the launcher becomes visible
    Component.onCompleted: {
        AppManager.ensureLoaded()
        root.filterApps(root.searchText)
    }

    onVisibleChanged: {
        if (visible) {
            AppManager.ensureLoaded()
            root.filterApps(root.searchText)
        }
    }

    Connections {
        target: AppManager
        function onAppsChanged() {
            root.filterApps(root.searchText)
        }
    }

    OverlayFocusScope {
        focusTarget: searchInput
    }

    function filterApps(text) {
        searchText = text.toLowerCase()
        filteredApps = AppManager.filter(searchText)
        selectedIndex = 0
    }

    function launchSelected() {
        if (filteredApps.length > 0 && selectedIndex >= 0 && selectedIndex < filteredApps.length) {
            AppManager.launch(filteredApps[selectedIndex].id)
            StateController.pillReset()
        }
    }

    function moveSelection(delta) {
        var len = filteredApps.length
        if (len === 0) return
        selectedIndex = (selectedIndex + delta + len) % len
    }

    function iconSource(iconName) {
        const icon = String(iconName || "").trim()
        if (icon.length === 0)
            return ""
        if (icon.indexOf("/") === 0)
            return "file://" + icon
        return Quickshell.iconPath(icon, "")
    }

    Keys.onPressed: function(event) {
        if (event.key === Qt.Key_Escape) {
            StateController.pillReset()
            event.accepted = true
        } else if (event.key === Qt.Key_Up) {
            moveSelection(-1)
            event.accepted = true
        } else if (event.key === Qt.Key_Down) {
            moveSelection(1)
            event.accepted = true
        } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            launchSelected()
            event.accepted = true
        }
    }

    // ── Panel body ────────────────────────────────────────────────────────
    Rectangle {
        anchors.fill: parent
        radius: root.containerRadius
        color: Theme.bg
        border.width: 1
        border.color: Theme.border
    }

    Column {
        anchors.fill: parent
        anchors.margins: root.padding
        spacing: root.gap

        // Header — title + live match count
        Item {
            width: parent.width
            height: root.headerHeight

            Txt {
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                text: "Applications"
                color: Theme.fg
                font.family: Theme.fontName
                font.pixelSize: 17
                font.bold: true
            }

            Txt {
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                text: root.filteredApps.length === AppManager.apps.length
                      ? root.filteredApps.length + " apps"
                      : root.filteredApps.length + " of " + AppManager.apps.length + " apps"
                color: Theme.fg3
                font.family: Theme.fontName
                font.pixelSize: 12
            }
        }

        // ── Search bar ─────────────────────────────────────────────────────
        Rectangle {
            width: parent.width
            height: root.searchHeight
            radius: 14
            color: Theme.bg2

            IconButton {
                id: searchIcon
                anchors.left: parent.left
                anchors.leftMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                glyph: "\uf002" // nf-mdi-magnify
                family: Theme.iconFont
                size: 26
                glyphSize: 17
                color: Theme.fg3
                enabled: false
                opacity: 0.9
            }

            Txt {
                anchors.left: searchIcon.right
                anchors.leftMargin: 10
                anchors.verticalCenter: parent.verticalCenter
                text: "Search apps…"
                color: Theme.fg3
                font.family: Theme.fontName
                font.pixelSize: 14
                visible: searchInput.text.length === 0
            }

            TxtInput {
                id: searchInput
                anchors.left: searchIcon.right
                anchors.leftMargin: 10
                anchors.right: clearIcon.left
                anchors.rightMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                color: Theme.fg
                font.family: Theme.fontName
                font.pixelSize: 14
                clip: true

                Keys.onUpPressed: function(event) {
                    root.moveSelection(-1)
                    event.accepted = true
                }
                Keys.onDownPressed: function(event) {
                    root.moveSelection(1)
                    event.accepted = true
                }
                Keys.onReturnPressed: function(event) {
                    root.launchSelected()
                    event.accepted = true
                }
                Keys.onEnterPressed: function(event) {
                    root.launchSelected()
                    event.accepted = true
                }
                Keys.onEscapePressed: function(event) {
                    StateController.pillReset()
                    event.accepted = true
                }

                onTextChanged: {
                    root.filterApps(text)
                }
            }

            IconButton {
                id: clearIcon
                anchors.right: parent.right
                anchors.rightMargin: 10
                anchors.verticalCenter: parent.verticalCenter
                glyph: "\uf00d" // nf-mdi-close
                family: Theme.iconFont
                size: 26
                glyphSize: 15
                color: Theme.fg3
                radius: 13
                borderWidth: 0
                opacity: searchInput.text.length > 0 ? 1 : 0
                enabled: searchInput.text.length > 0

                Behavior on opacity {
                    NumberAnimation { duration: 120 }
                }

                onClicked: {
                    searchInput.text = ""
                    searchInput.forceActiveFocus()
                }
            }
        }

        // ── App list ───────────────────────────────────────────────────────
        Item {
            width: parent.width
            height: root.listAreaHeight

            ListView {
                id: appList
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                anchors.right: parent.right
                anchors.rightMargin: 8
                clip: true
                model: root.filteredApps
                currentIndex: root.selectedIndex
                spacing: root.itemSpacing

                delegate: Item {
                    id: appDelegate
                    required property int index
                    required property var modelData
                    width: appList.width
                    height: root.itemHeight

                    Rectangle {
                        anchors.fill: parent
                        radius: root.itemRounding
                        color: appList.currentIndex === appDelegate.index ? Theme.acc : "transparent"
                        border.width: appList.currentIndex === appDelegate.index ? 1 : 0
                        border.color: Theme.acc

                        Behavior on color {
                            ColorAnimation { duration: 120 }
                        }
                    }

                    Row {
                        anchors.fill: parent
                        anchors.leftMargin: root.padding
                        anchors.rightMargin: root.padding
                        spacing: 12

                        // Icon — theme-resolved image with glyph fallback
                        Rectangle {
                            width: 32
                            height: 32
                            radius: 8
                            anchors.verticalCenter: parent.verticalCenter
                            color: appList.currentIndex === appDelegate.index ? Theme.bg : Theme.bg2

                            Image {
                                id: appIcon
                                anchors.fill: parent
                                anchors.margins: 2
                                source: root.iconSource(appDelegate.modelData.icon)
                                fillMode: Image.PreserveAspectFit
                                asynchronous: true
                                visible: status === Image.Ready
                            }

                            Txt {
                                anchors.centerIn: parent
                                visible: appIcon.status !== Image.Ready
                                text: "\uf2e8" // nf-mdi-apps
                                color: appList.currentIndex === appDelegate.index ? Theme.sfg : Theme.fg3
                                font.family: Theme.iconFont
                                font.pixelSize: 17
                            }
                        }

                        Column {
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 2
                            width: parent.width - 32 - 12

                            Txt {
                                text: appDelegate.modelData.name
                                color: appList.currentIndex === appDelegate.index ? Theme.sfg : Theme.fg
                                font.family: Theme.fontName
                                font.pixelSize: 15
                                font.bold: true
                                width: parent.width
                                elide: Text.ElideRight

                                Behavior on color {
                                    ColorAnimation { duration: 120 }
                                }
                            }

                            Txt {
                                text: appDelegate.modelData.id
                                color: appList.currentIndex === appDelegate.index ? Theme.sfg : Theme.fg3
                                font.family: Theme.fontName
                                font.pixelSize: 12
                                width: parent.width
                                elide: Text.ElideRight

                                Behavior on color {
                                    ColorAnimation { duration: 120 }
                                }
                            }
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onEntered: {
                            root.selectedIndex = appDelegate.index
                        }
                        onPositionChanged: {
                            root.selectedIndex = appDelegate.index
                        }
                        onClicked: {
                            root.selectedIndex = appDelegate.index
                            root.launchSelected()
                        }
                    }
                }

                onCurrentIndexChanged: {
                    positionViewAtIndex(currentIndex, ListView.Contain)
                }
            }

            ScrollDragger {
                target: appList
            }

            EmptyState {
                anchors.centerIn: parent
                visible: root.filteredApps.length === 0
                icon: "\uf1c0" // nf-mdi-database-search
                text: "No apps match “" + searchInput.text + "”"
            }
        }
    }
}
