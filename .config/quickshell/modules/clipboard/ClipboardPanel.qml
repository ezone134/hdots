import QtQuick
import Quickshell
import "."
import "../../components"
import "../../services"

Item {
    id: root
    implicitWidth: 520
    implicitHeight: root.imageMode ? 440 : 400
    focus: true

    property bool imageMode: false
    property var filteredEntries: []
    property string searchText: ""
    property int selectedIndex: 0
    property bool thumbsReady: ClipboardManager.thumbsReady

    Component.onCompleted: {
        ClipboardManager.refresh(root.imageMode ? "image" : "text")
        root.refilter()
    }

    onVisibleChanged: {
        if (visible) {
            ClipboardManager.refresh(root.imageMode ? "image" : "text")
            root.refilter()
        }
    }

    Connections {
        target: ClipboardManager

        function onTextEntriesChanged() {
            if (!root.imageMode) root.refilter()
        }

        function onImageEntriesChanged() {
            if (root.imageMode) root.refilter()
        }
    }

    OverlayFocusScope {
        focusTarget: root.imageMode ? root : searchInput
    }

    // Content for display/search: everything after the first tab,
    // newlines flattened for single-line rows.
    function entryContent(line) {
        var s = String(line || "")
        var tab = s.indexOf("\t")
        if (tab >= 0) s = s.substring(tab + 1)
        return s.replace(/\n/g, " ").trim()
    }

    function entryIndex(line) {
        var s = String(line || "")
        var tab = s.indexOf("\t")
        return tab > 0 ? s.substring(0, tab) : s
    }

    function refilter() {
        const src = root.imageMode ? ClipboardManager.imageEntries : ClipboardManager.textEntries
        const q = root.searchText.toLowerCase()
        if (root.imageMode || q.length === 0) {
            root.filteredEntries = src.slice()
        } else {
            root.filteredEntries = src.filter(line => root.entryContent(line).toLowerCase().indexOf(q) !== -1)
        }
        root.selectedIndex = 0
    }

    function copySelected() {
        if (root.filteredEntries.length === 0)
            return
        const line = root.filteredEntries[root.selectedIndex]
        ClipboardManager.copy(line, root.imageMode ? "image" : "text")
        StateController.pillReset()
    }

    function removeSelected() {
        if (root.filteredEntries.length === 0)
            return
        const line = root.filteredEntries[root.selectedIndex]
        ClipboardManager.remove(line, root.imageMode ? "image" : "text")
    }

    function moveSelection(delta) {
        const len = root.filteredEntries.length
        if (len === 0)
            return
        root.selectedIndex = (root.selectedIndex + delta + len) % len
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
            root.copySelected()
            event.accepted = true
        } else if (event.key === Qt.Key_Delete) {
            root.removeSelected()
            event.accepted = true
        }
    }

    Column {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 10

        // Header
        Row {
            width: parent.width
            spacing: 12

            Rectangle {
                width: 38
                height: 38
                radius: 19
                color: Theme.bg3

                Txt {
                    anchors.centerIn: parent
                    text: root.imageMode ? "󰥶" : "󰆏"
                    font.family: Theme.iconFont
                    font.pixelSize: 19
                    color: Theme.fg
                }
            }

            Column {
                anchors.verticalCenter: parent.verticalCenter
                spacing: 1

                Txt {
                    text: root.imageMode ? "Clipboard — Images" : "Clipboard"
                    color: Theme.fg
                    font.family: Theme.fontName
                    font.pixelSize: 15
                    font.bold: true
                }

                Txt {
                    text: root.imageMode
                        ? (root.filteredEntries.length > 0 ? root.filteredEntries.length + " images • Enter: copy • Del: remove" : "Enter: copy • Del: remove")
                        : (root.filteredEntries.length > 0 ? root.filteredEntries.length + " entries • Enter: copy • Del: remove" : "Enter: copy • Del: remove")
                    color: Theme.fg3
                    font.family: Theme.fontName
                    font.pixelSize: 11
                }
            }
        }

        // Search (text mode only)
        Rectangle {
            id: searchBox
            visible: !root.imageMode
            width: parent.width
            height: 40
            radius: 10
            color: Theme.bg3
            border.color: searchInput.activeFocus ? Theme.acc : "transparent"
            border.width: 1

            Behavior on color {
                ColorAnimation { duration: 120 }
            }

            Behavior on border.color {
                ColorAnimation { duration: 120 }
            }

            Txt {
                anchors.left: parent.left
                anchors.leftMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                text: "Search clipboard..."
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
                    root.copySelected()
                    event.accepted = true
                }
                Keys.onEnterPressed: function(event) {
                    root.copySelected()
                    event.accepted = true
                }
                Keys.onEscapePressed: function(event) {
                    StateController.pillReset()
                    event.accepted = true
                }

                onTextChanged: {
                    root.searchText = text
                    root.refilter()
                }
            }
        }

        // Entry list
        Item {
            width: parent.width
            height: parent.height - (root.imageMode ? 58 : 100)

            Txt {
                anchors.centerIn: parent
                visible: root.filteredEntries.length === 0
                text: root.imageMode ? "No images in clipboard history" : "Clipboard is empty"
                color: Theme.fg3
                font.family: Theme.fontName
                font.pixelSize: 13
            }

            ListView {
                id: clipList
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                anchors.right: parent.right
                anchors.rightMargin: 8
                clip: true
                model: root.filteredEntries
                currentIndex: root.selectedIndex

                delegate: Item {
                    required property int index
                    required property var modelData
                    width: clipList.width
                    height: root.imageMode ? 60 : 36

                    Rectangle {
                        anchors.fill: parent
                        radius: root.imageMode ? 10 : 8
                        color: clipList.currentIndex === index ? Theme.acc : "transparent"
                        border.color: clipList.currentIndex === index ? Theme.acc : "transparent"
                        border.width: clipList.currentIndex === index ? 1 : 0

                        Behavior on color {
                            ColorAnimation { duration: 120 }
                        }

                        Row {
                            anchors.fill: parent
                            anchors.margins: 8
                            spacing: 12

                            Rectangle {
                                width: root.imageMode ? 44 : 20
                                height: root.imageMode ? 44 : 20
                                radius: root.imageMode ? 8 : 4
                                color: clipList.currentIndex === index ? Theme.bg : (root.imageMode ? Theme.bg3 : "transparent")
                                anchors.verticalCenter: parent.verticalCenter

                                Image {
                                    id: thumbImage
                                    anchors.fill: parent
                                    anchors.margins: root.imageMode ? 2 : 3
                                    visible: root.imageMode && root.thumbsReady
                                    source: root.imageMode ? "file:///tmp/cliphist_thumbs/" + root.entryIndex(modelData) + ".img" : ""
                                    fillMode: Image.PreserveAspectFit
                                    asynchronous: true
                                    smooth: true
                                }

                                Txt {
                                    anchors.centerIn: parent
                                    visible: !root.imageMode || !root.thumbsReady || thumbImage.status !== Image.Ready
                                    text: root.imageMode ? "󰥶" : "󰆏"
                                    font.family: Theme.iconFont
                                    font.pixelSize: root.imageMode ? 18 : 10
                                    color: clipList.currentIndex === index ? Theme.sfg : Theme.fg3
                                }
                            }

                            Column {
                                anchors.verticalCenter: parent.verticalCenter
                                width: parent.width - 90
                                spacing: root.imageMode ? 2 : 0

                                Txt {
                                    text: root.imageMode ? "Image" : root.entryContent(modelData)
                                    color: clipList.currentIndex === index ? Theme.sfg : Theme.fg
                                    font.family: Theme.fontName
                                    font.pixelSize: root.imageMode ? 13 : 13
                                    font.bold: root.imageMode || clipList.currentIndex === index
                                    width: parent.width
                                    elide: Text.ElideRight

                                    Behavior on color {
                                        ColorAnimation { duration: 120 }
                                    }
                                }

                                Txt {
                                    visible: root.imageMode
                                    text: "entry #" + root.entryIndex(modelData)
                                    color: clipList.currentIndex === index ? Theme.sfg : Theme.fg3
                                    font.family: Theme.fontName
                                    font.pixelSize: 11
                                    opacity: clipList.currentIndex === index ? 0.9 : 0.8
                                }
                            }
                        }

                        MouseArea {
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onEntered: root.selectedIndex = index
                            onPositionChanged: root.selectedIndex = index
                            onClicked: {
                                root.selectedIndex = index
                                root.copySelected()
                            }
                        }
                    }
                }

                onCurrentIndexChanged: {
                    positionViewAtIndex(currentIndex, ListView.Contain)
                }
            }

            ScrollDragger {

                target: clipList

            }
        }
    }
}