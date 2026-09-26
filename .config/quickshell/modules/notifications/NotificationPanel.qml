pragma ComponentBehavior: Bound
import QtQuick
import "."
import "../../components"
import "../../services"

Item {
    id: root
    implicitWidth: 420
    implicitHeight: 520
    focus: true

    property int selectedIndex: NotificationManager.history.length > 0 ? 0 : -1
    property bool groupByApp: false
    // "history" renders the (optionally grouped) history list; "blocked"
    // lists the muted apps so they can be unblocked. Toggled from the
    // header's bell-off button and the 'B' key.
    property string viewMode: "history"

    function currentList() {
        if (!groupByApp)
            return NotificationManager.history

        const flattened = []
        const groups = NotificationManager.groupedHistory()
        for (let i = 0; i < groups.length; i++) {
            flattened.push({
                type: "group-header",
                title: groups[i].appName,
                subtitle: groups[i].category,
                count: groups[i].items.length,
                groupKey: groups[i].appName + "::" + groups[i].category
            })
            for (let j = 0; j < groups[i].items.length; j++)
                flattened.push(groups[i].items[j])
        }
        return flattened
    }

    function currentModel() {
        return root.viewMode === "blocked" ? root.blockedModel() : root.currentList()
    }

    function blockedModel() {
        const out = []
        for (let i = 0; i < NotificationManager.blockedApps.length; i++)
            out.push({ type: "blocked-row", appName: NotificationManager.blockedApps[i] })
        return out
    }

    function moveSelection(delta) {
        const len = currentModel().length
        if (len <= 0)
            return
        selectedIndex = (selectedIndex + delta + len) % len
    }

    onViewModeChanged: {
        const m = root.viewMode === "blocked" ? root.blockedModel() : root.currentList()
        selectedIndex = m.length > 0 ? 0 : -1
    }

    function removeSelected() {
        const items = currentList()
        if (selectedIndex < 0 || selectedIndex >= items.length)
            return
        const item = items[selectedIndex]
        if (item.type === "group-header")
            return
        NotificationManager.removeHistoryNotification(item.id)
        if (NotificationManager.history.length === 0)
            selectedIndex = -1
        else if (selectedIndex >= NotificationManager.history.length)
            selectedIndex = NotificationManager.history.length - 1
    }

    Keys.onPressed: function(event) {
        if (event.key === Qt.Key_Escape) {
            NotificationManager.markAllRead()
            StateController.pillReset()
            event.accepted = true
        } else if (event.key === Qt.Key_Up) {
            moveSelection(-1)
            event.accepted = true
        } else if (event.key === Qt.Key_Down) {
            moveSelection(1)
            event.accepted = true
        } else if (event.key === Qt.Key_Delete || event.key === Qt.Key_Backspace) {
            if (root.viewMode === "blocked") {
                const rows = root.blockedModel()
                if (rows.length > 0 && root.selectedIndex >= 0 && root.selectedIndex < rows.length)
                    NotificationManager.unblockApp(rows[root.selectedIndex].appName)
                if (rows.length <= 1)
                    root.selectedIndex = -1
                else if (root.selectedIndex >= rows.length - 1)
                    root.selectedIndex = rows.length - 2
            } else {
                removeSelected()
            }
            event.accepted = true
        } else if (event.key === Qt.Key_B) {
            // Mute/unmute the selected item's app (not on group headers).
            if (root.viewMode !== "blocked") {
                const items = root.currentList()
                const sel = items[root.selectedIndex]
                if (sel && sel.type !== "group-header") {
                    NotificationManager.toggleBlockedApp(sel.appName)
                    if (root.selectedIndex >= root.currentList().length)
                        root.selectedIndex = Math.max(0, root.currentList().length - 1)
                }
            }
            event.accepted = true
        }
    }

    Rectangle {
        anchors.fill: parent
        radius: 20
        color: Theme.bg
    }

    Column {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 10

        Row {
            width: parent.width
            spacing: 10

            Txt {
                text: "Notifications"
                color: Theme.fg
                font.family: Theme.fontName
                font.pixelSize: 18
                font.bold: true
            }

            Rectangle {
                width: unreadText.implicitWidth + 16
                height: 24
                radius: 12
                color: Theme.acc
                visible: NotificationManager.unreadCount > 0

                Txt {
                    id: unreadText
                    anchors.centerIn: parent
                    text: String(NotificationManager.unreadCount)
                    color: Theme.sfg
                    font.family: Theme.fontName
                    font.pixelSize: 11
                    font.bold: true
                }
            }

            Item {
                width: parent.width - dndButton.width - groupButton.width - blockedButton.width - clearVisibleButton.width - clearHistoryButton.width - 100
                height: 1
            }

            Rectangle {
                id: dndButton
                width: 62
                height: 28
                radius: 14
                color: NotificationManager.dndEnabled ? Theme.acc : Theme.bg3

                Txt {
                    anchors.centerIn: parent
                    text: "DND"
                    color: NotificationManager.dndEnabled ? Theme.sfg : Theme.bg
                    font.family: Theme.fontName
                    font.pixelSize: 12
                    font.bold: true
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: NotificationManager.toggleDnd()
                }
            }

            Rectangle {
                id: groupButton
                width: 70
                height: 28
                radius: 14
                color: root.groupByApp ? Theme.acc : Theme.bg3

                Txt {
                    anchors.centerIn: parent
                    text: "Group"
                    color: root.groupByApp ? Theme.sfg : Theme.bg
                    font.family: Theme.fontName
                    font.pixelSize: 12
                    font.bold: true
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: root.groupByApp = !root.groupByApp
                }
            }

            Rectangle {
                id: blockedButton
                width: 34
                height: 28
                radius: 14
                color: root.viewMode === "blocked" ? Theme.danger : Theme.bg3

                Txt {
                    anchors.centerIn: parent
                    text: "\uf2d0"
                    color: root.viewMode === "blocked" ? Theme.sfg : Theme.bg
                    font.family: Theme.fontName
                    font.pixelSize: 13
                    font.bold: true
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: root.viewMode = root.viewMode === "blocked" ? "history" : "blocked"
                }
            }

            Rectangle {
                id: clearVisibleButton
                width: 78
                height: 28
                radius: 14
                color: Theme.acc

                Txt {
                    anchors.centerIn: parent
                    text: "Clear"
                    color: Theme.sfg
                    font.family: Theme.fontName
                    font.pixelSize: 12
                    font.bold: true
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: NotificationManager.clearAll()
                }
            }

            Rectangle {
                id: clearHistoryButton
                width: 90
                height: 28
                radius: 14
                color: Theme.bg3

                Txt {
                    anchors.centerIn: parent
                    text: "History"
                    color: Theme.bg
                    font.family: Theme.fontName
                    font.pixelSize: 12
                    font.bold: true
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: NotificationManager.clearHistory()
                }
            }
        }

        Txt {
            text: root.viewMode === "blocked"
                ? (NotificationManager.blockedApps.length > 0
                    ? NotificationManager.blockedApps.length + " muted • Delete to unblock"
                    : "No blocked apps")
                : (NotificationManager.history.length > 0
                    ? NotificationManager.history.length + " items • " + (NotificationManager.dndEnabled ? "DND on" : "DND off")
                    : "No notifications")
            color: Theme.fg3
            font.family: Theme.fontName
            font.pixelSize: 12
        }

        Item {
            width: parent.width
            height: parent.height - 62

            Component {
                id: groupHeaderCard

                Rectangle {
                    property var modelData
                    property int modelIndex
                    width: historyList.width
                    height: 34
                    radius: 10
                    color: Theme.bg3

                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        onEntered: root.selectedIndex = modelIndex
                        onPositionChanged: root.selectedIndex = modelIndex
                        onClicked: root.selectedIndex = modelIndex
                    }

                    Row {
                        anchors.fill: parent
                        anchors.leftMargin: 12
                        anchors.rightMargin: 8
                        spacing: 8

                        Txt {
                            anchors.verticalCenter: parent.verticalCenter
                            text: modelData.title
                            color: Theme.bg
                            font.family: Theme.fontName
                            font.pixelSize: 12
                            font.bold: true
                            width: parent.width - 90
                            elide: Text.ElideRight
                        }

                        Txt {
                            anchors.verticalCenter: parent.verticalCenter
                            text: modelData.count + ""
                            color: Theme.bg
                            opacity: 0.8
                            font.family: Theme.fontName
                            font.pixelSize: 11
                        }

                        Item {
                            width: 22
                            height: 22
                            Rectangle {
                                anchors.fill: parent
                                radius: 11
                                color: blockHover.containsMouse ? Theme.danger : "transparent"

                                Behavior on color {
                                    ColorAnimation { duration: 100 }
                                }

                                Txt {
                                    anchors.centerIn: parent
                                    text: "\uf2d0"
                                    font.family: Theme.fontName
                                    font.pixelSize: 12
                                    color: Theme.bg
                                    opacity: blockHover.containsMouse ? 1 : 0.75
                                }
                            }
                            MouseArea {
                                id: blockHover
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    NotificationManager.toggleBlockedApp(modelData.title)
                                    if (root.selectedIndex >= root.currentList().length)
                                        root.selectedIndex = Math.max(0, root.currentList().length - 1)
                                }
                            }
                        }
                    }
                }
            }

            Component {
                id: blockedAppCard

                Rectangle {
                    property var modelData
                    property int modelIndex
                    width: historyList.width
                    height: 44
                    radius: 10
                    color: historyList.currentIndex === modelIndex ? Theme.acc : Theme.bg3
                    border.color: historyList.currentIndex === modelIndex ? Theme.acc : "transparent"
                    border.width: historyList.currentIndex === modelIndex ? 1 : 0

                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        onEntered: root.selectedIndex = modelIndex
                        onPositionChanged: root.selectedIndex = modelIndex
                        onClicked: root.selectedIndex = modelIndex
                    }

                    Row {
                        anchors.fill: parent
                        anchors.leftMargin: 12
                        anchors.rightMargin: 8
                        spacing: 10

                        Txt {
                            anchors.verticalCenter: parent.verticalCenter
                            text: "\uf2d0"
                            font.family: Theme.fontName
                            font.pixelSize: 14
                            color: historyList.currentIndex === modelIndex ? Theme.sfg : Theme.danger
                        }

                        Txt {
                            anchors.verticalCenter: parent.verticalCenter
                            text: modelData.appName
                            color: historyList.currentIndex === modelIndex ? Theme.sfg : Theme.fg
                            font.family: Theme.fontName
                            font.pixelSize: 13
                            font.bold: true
                            elide: Text.ElideRight
                            width: parent.width - 180
                        }

                        Txt {
                            anchors.verticalCenter: parent.verticalCenter
                            text: "blocked"
                            color: historyList.currentIndex === modelIndex ? Theme.sfg : Theme.fg3
                            font.family: Theme.fontName
                            font.pixelSize: 11
                        }

                        Rectangle {
                            anchors.verticalCenter: parent.verticalCenter
                            width: 64
                            height: 26
                            radius: 13
                            color: Theme.acc

                            Txt {
                                anchors.centerIn: parent
                                text: "Unblock"
                                color: Theme.sfg
                                font.family: Theme.fontName
                                font.pixelSize: 11
                                font.bold: true
                            }

                            MouseArea {
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: NotificationManager.unblockApp(modelData.appName)
                            }
                        }
                    }
                }
            }

            Component {
                id: notificationHistoryCard

                Rectangle {
                    id: card
                    property var modelData
                    property int modelIndex
                    property var style: modelData.style || ({
                        backgroundColor: Theme.bg,
                        borderColor: Theme.border,
                        textColor: Theme.fg,
                        fontFamily: Theme.fontName,
                        borderRadius: 10,
                        borderSize: 1,
                        width: historyList.width,
                        fontPixelSize: 13,
                        titleFontPixelSize: 13,
                        appFontPixelSize: 12,
                        bodyFontPixelSize: 13,
                        textAlignment: "center"
                    })
                    width: historyList.width
                    radius: Number(style.borderRadius || 10)
                    color: style.backgroundColor || Theme.bg
                    border.width: Number(style.borderSize || 2)
                    border.color: historyList.currentIndex === modelIndex ? Theme.acc : (style.borderColor || Theme.bg3)
                    implicitHeight: contentColumn.implicitHeight + 18

                    Column {
                        id: contentColumn
                        anchors.fill: parent
                        anchors.margins: 10
                        spacing: 6

                        Txt {
                            text: modelData.summary.length > 0 ? modelData.summary : modelData.appName
                            color: historyList.currentIndex === modelIndex ? Theme.sfg : (style.textColor || Theme.fg)
                            font.family: style.fontFamily || Theme.fontName
                            font.pixelSize: Number(style.titleFontPixelSize || style.fontPixelSize || 13)
                            font.bold: true
                            width: parent.width
                            wrapMode: Text.Wrap
                            horizontalAlignment: Text.AlignHCenter
                        }

                        Txt {
                            visible: modelData.appName.length > 0 && modelData.summary !== modelData.appName
                            text: modelData.appName
                            color: historyList.currentIndex === modelIndex ? Theme.sfg : (style.textColor || Theme.fg)
                            opacity: 0.8
                            font.family: style.fontFamily || Theme.fontName
                            font.pixelSize: Number(style.appFontPixelSize || 12)
                            width: parent.width
                            wrapMode: Text.Wrap
                            horizontalAlignment: Text.AlignHCenter
                        }

                        Txt {
                            visible: modelData.body.length > 0
                            text: modelData.body
                            color: historyList.currentIndex === modelIndex ? Theme.sfg : (style.textColor || Theme.fg)
                            font.family: style.fontFamily || Theme.fontName
                            font.pixelSize: Number(style.bodyFontPixelSize || style.fontPixelSize || 13)
                            width: parent.width
                            wrapMode: Text.Wrap
                            horizontalAlignment: Text.AlignHCenter
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        onEntered: root.selectedIndex = modelIndex
                        onPositionChanged: root.selectedIndex = modelIndex
                        onClicked: root.selectedIndex = modelIndex
                    }
                }
            }

            ListView {
                id: historyList
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                anchors.right: parent.right
                anchors.rightMargin: 8
                clip: true
                model: root.viewMode === "blocked" ? root.blockedModel() : root.currentList()
                currentIndex: root.selectedIndex

                delegate: Item {
                    id: historyDelegate
                    required property int index
                    required property var modelData
                    width: historyList.width
                    height: (rowLoader.item ? rowLoader.item.implicitHeight : 34) + 6

                    Loader {
                        id: rowLoader
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        sourceComponent: historyDelegate.modelData.type === "group-header" ? groupHeaderCard
                                    : historyDelegate.modelData.type === "blocked-row" ? blockedAppCard
                                    : notificationHistoryCard
                        onLoaded: {
                            item.modelData = historyDelegate.modelData
                            item.modelIndex = historyDelegate.index
                        }
                    }
                }

                onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
            }

            ScrollDragger {

                target: historyList

            }
        }
    }
}