pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import "."
import "../../components"
import "../../services"

Item {
    id: root

    property var rows: HyprlandManager.workspaceRowModel()
    property int selectedRow: 0
    property int selectedColumn: 0
    property int maxVisibleWindows: 12
    property bool awaitingRelease: false
    property real contentOpacity: 0
    property real contentScale: 0.98

    readonly property int rowHeight: 74
    readonly property int rowSpacing: 12
    readonly property int chipHeight: 52
    readonly property int chipSpacing: 10
    readonly property int rowLabelWidth: 60
    readonly property int contentPadding: 16
    readonly property int visibleRowCount: Math.min(rows.length, 2)
    readonly property int bodyWidth: 720

    implicitWidth: bodyWidth
    implicitHeight: 270

    focus: true
    Keys.priority: Keys.BeforeItem

    function clampRow(index) {
        if (rows.length === 0)
            return 0;
        return Math.max(0, Math.min(index, rows.length - 1));
    }

    function selectedWindows(rowIndex) {
        if (rowIndex < 0 || rowIndex >= rows.length)
            return [];
        return rows[rowIndex].windows || [];
    }

    function clampColumn(rowIndex, column) {
        const windows = selectedWindows(rowIndex);
        const workspaceSlotCount = Math.max(1, windows.length);
        return Math.max(0, Math.min(column, workspaceSlotCount - 1));
    }

    function syncSelection() {
        selectedRow = clampRow(selectedRow);
        selectedColumn = clampColumn(selectedRow, selectedColumn);
    }

    function closeSwitcher() {
        awaitingRelease = true;
        StateController.returnToPill();
    }

    function activateSelection() {
        if (rows.length === 0)
            return;
        const row = rows[selectedRow];
        if (!row)
            return;
        const windows = row.windows || [];
        if (windows.length === 0) {
            HyprlandManager.focusWorkspace(row.workspaceId);
            closeSwitcher();
            return;
        }
        const target = windows[clampColumn(selectedRow, selectedColumn)];
        if (target && target.address.length > 0)
            HyprlandManager.focusWindow(target.address);
        else
            HyprlandManager.focusWorkspace(row.workspaceId);
        closeSwitcher();
    }

    Keys.onPressed: event => {
        if (awaitingRelease) {
            event.accepted = true;
            return;
        }

        const hasMod = (event.modifiers & Qt.MetaModifier) !== 0;

        if (event.key === Qt.Key_Escape) {
            closeSwitcher();
            event.accepted = true;
        } else if (event.key === Qt.Key_Tab && hasMod) {
            activateSelection();
            event.accepted = true;
        } else if (event.key === Qt.Key_Left && hasMod) {
            selectedColumn = clampColumn(selectedRow, selectedColumn - 1);
            event.accepted = true;
        } else if (event.key === Qt.Key_Right && hasMod) {
            selectedColumn = clampColumn(selectedRow, selectedColumn + 1);
            event.accepted = true;
        } else if (event.key === Qt.Key_Up && hasMod) {
            selectedRow = clampRow(selectedRow - 1);
            selectedColumn = clampColumn(selectedRow, selectedColumn);
            event.accepted = true;
        } else if (event.key === Qt.Key_Down && hasMod) {
            selectedRow = clampRow(selectedRow + 1);
            selectedColumn = clampColumn(selectedRow, selectedColumn);
            event.accepted = true;
        } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Space) {
            activateSelection();
            event.accepted = true;
        } else {
            event.accepted = true;
        }
    }

    Keys.onReleased: event => {
        if (awaitingRelease) {
            const hasMod = (event.modifiers & Qt.MetaModifier) !== 0;
            if (!hasMod)
                awaitingRelease = false;
            event.accepted = true;
            return;
        }
        event.accepted = true;
    }

    onRowsChanged: syncSelection()
    Component.onCompleted: {
        syncSelection();
        forceActiveFocus();
    }

    Timer {
        id: contentInTimer
        interval: 80
        repeat: false
        running: true
        onTriggered: {
            root.contentOpacity = 1
            root.contentScale = 1
        }
    }

    Item {
        anchors.fill: parent
        opacity: root.contentOpacity
        scale: root.contentScale

        Behavior on opacity {
            NumberAnimation { duration: 200; easing.type: Easing.OutCubic }
        }
        Behavior on scale {
            NumberAnimation { duration: 200; easing.type: Easing.OutCubic }
        }

        Column {
            anchors.fill: parent
            anchors.margins: root.contentPadding
            spacing: 12

            Txt {
                anchors.horizontalCenter: parent.horizontalCenter
                text: "Workspaces"
                color: Theme.fg
                font.family: Theme.fontName
                font.pixelSize: 18
                font.bold: true
            }

            Item {
                width: parent.width
                height: parent.height - 30

                Column {
                    anchors.centerIn: parent
                    width: parent.width
                    spacing: root.rowSpacing

                    Repeater {
                        model: root.rows.slice(0, 2)

                        delegate: Row {
                            id: workspaceRow
                            required property int index
                            required property var modelData
                            property int rowIndex: index
                            property bool isActiveWorkspace: workspaceRow.modelData.workspaceId === HyprlandManager.currentWorkspace
                            property bool rowSelected: root.selectedRow === workspaceRow.rowIndex
                            spacing: 12
                            width: parent.width
                            height: root.rowHeight

                            Rectangle {
                                width: root.rowLabelWidth
                                height: root.rowHeight
                                radius: 22
                                color: workspaceRow.rowSelected ? Theme.bg3 : (workspaceRow.isActiveWorkspace ? Theme.acc : Qt.rgba(0, 0, 0, 0.06))

                                Column {
                                    anchors.centerIn: parent
                                    spacing: 2

                                    Txt {
                                        anchors.horizontalCenter: parent.horizontalCenter
                                        text: String(workspaceRow.modelData.workspaceId)
                                        color: workspaceRow.rowSelected ? Theme.bg : (workspaceRow.isActiveWorkspace ? Theme.sfg : Theme.fg)
                                        font.bold: true
                                        font.pixelSize: 22
                                    }

                                    Txt {
                                        anchors.horizontalCenter: parent.horizontalCenter
                                        text: "ws"
                                        color: workspaceRow.rowSelected ? Theme.bg : Theme.fg3
                                        font.pixelSize: 10
                                    }
                                }
                            }

                            Flickable {
                                id: rowFlickable
                                width: root.bodyWidth - root.contentPadding * 2 - root.rowLabelWidth - 12
                                height: root.rowHeight
                                clip: true
                                contentWidth: windowRow.width
                                contentHeight: root.rowHeight
                                boundsBehavior: Flickable.StopAtBounds

                                Row {
                                    id: windowRow
                                    spacing: root.chipSpacing
                                    height: root.rowHeight

                                    Repeater {
                                        model: workspaceRow.modelData.windows.length > 0 ? workspaceRow.modelData.windows : [{
                                            address: "",
                                            title: "Empty workspace",
                                            className: "Workspace",
                                            workspaceId: workspaceRow.modelData.workspaceId
                                        }]

                                        delegate: Rectangle {
                                            id: windowChip
                                            required property int index
                                            required property var modelData
                                            property bool isPlaceholder: windowChip.modelData.address === ""
                                            property bool isSelected: root.selectedRow === workspaceRow.rowIndex && root.selectedColumn === windowChip.index
                                            width: Math.max(140, Math.min(230, Math.max(titleText.implicitWidth + 34, classText.implicitWidth + 34)))
                                            height: root.chipHeight
                                            radius: 18
                                            anchors.verticalCenter: windowRow.verticalCenter
                                            color: windowChip.isSelected
                                                ? Theme.acc
                                                : Qt.rgba(0, 0, 0, workspaceRow.rowSelected ? 0.08 : 0.04)
                                            border.width: windowChip.modelData.workspaceId === HyprlandManager.currentWorkspace ? 1 : 0
                                            border.color: windowChip.isSelected ? Theme.acc : Qt.rgba(1, 1, 1, 0.4)

                                            Behavior on color {
                                                ColorAnimation { duration: 140 }
                                            }

                                            Column {
                                                anchors.fill: parent
                                                anchors.leftMargin: 14
                                                anchors.rightMargin: 14
                                                anchors.verticalCenter: parent.verticalCenter
                                                spacing: 0

                                                Txt {
                                                    id: classText
                                                    text: windowChip.modelData.className
                                                    color: windowChip.isSelected ? Theme.sfg : Theme.fg3
                                                    font.pixelSize: 11
                                                    font.bold: true
                                                    elide: Text.ElideRight
                                                    width: parent.width
                                                }

                                                Txt {
                                                    id: titleText
                                                    text: windowChip.modelData.title
                                                    color: windowChip.isSelected ? Theme.sfg : (windowChip.isPlaceholder ? Theme.fg3 : Theme.fg)
                                                    font.pixelSize: 14
                                                    font.bold: windowChip.isSelected
                                                    elide: Text.ElideRight
                                                    width: parent.width
                                                }
                                            }

                                            Rectangle {
                                                visible: windowChip.isSelected
                                                anchors.top: parent.top
                                                anchors.horizontalCenter: parent.horizontalCenter
                                                anchors.topMargin: 8
                                                width: 28
                                                height: 4
                                                radius: 2
                                                color: Theme.sfg
                                                opacity: 0.9
                                            }

                                            MouseArea {
                                                anchors.fill: parent
                                                hoverEnabled: true
                                                onEntered: {
                                                    root.selectedRow = workspaceRow.rowIndex;
                                                    root.selectedColumn = windowChip.index;
                                                }
                                                onClicked: {
                                                    root.selectedRow = workspaceRow.rowIndex;
                                                    root.selectedColumn = windowChip.index;
                                                    root.activateSelection();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Row {
                        anchors.horizontalCenter: parent.horizontalCenter
                        spacing: 14

                        Txt {
                            text: "Super + ← → move"
                            color: Theme.fg3
                            font.family: Theme.fontName
                            font.pixelSize: 12
                        }

                        Txt {
                            text: "Super + ↑ ↓ row"
                            color: Theme.fg3
                            font.family: Theme.fontName
                            font.pixelSize: 12
                        }

                        Txt {
                            text: "Enter select"
                            color: Theme.fg3
                            font.family: Theme.fontName
                            font.pixelSize: 12
                        }
                    }
                }
            }
        }
    }
}