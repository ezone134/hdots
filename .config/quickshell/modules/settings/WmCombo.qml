import QtQuick
import "../../components"

// Combo row for WmCategory: label + hint on the left, a chip showing the
// current selection on the right. Clicking the chip expands an in-flow
// dropdown (capped height, scrollable via ScrollDragger). Selection is
// committed as the entry index via changed(index) so the category can write
// the state file and run the restore.
Item {
    id: root

    property string label: ""
    property string hint: ""
    property var entries: []
    property int currentIndex: -1
    signal changed(int index)

    property bool open: false
    property int _selIndex: -1

    implicitHeight: root.open ? 46 + 6 + root._listHeight : 46
    width: parent ? parent.width : 0

    readonly property int _listHeight: Math.min(216, Math.max(42, (root.entries.length * 34) + 8))

    function _toggle() {
        root.open = !root.open
        if (root.open) {
            root._selIndex = root.currentIndex >= 0 ? root.currentIndex : 0
            root.forceActiveFocus()
        }
    }

    function _move(delta) {
        const n = root.entries.length
        if (n === 0)
            return
        root._selIndex = (root._selIndex + delta + n) % n
        listView.positionViewAtIndex(root._selIndex, ListView.Contain)
    }

    function _commit() {
        if (root._selIndex >= 0)
            root.changed(root._selIndex)
        root.open = false
    }

    Keys.onPressed: event => {
        if (event.key === Qt.Key_Escape && root.open) {
            root.open = false
            event.accepted = true
        } else if ((event.key === Qt.Key_Down || event.key === Qt.Key_Up) && root.open) {
            root._move(event.key === Qt.Key_Down ? 1 : -1)
            event.accepted = true
        } else if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter) && root.open) {
            root._commit()
            event.accepted = true
        } else if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Space) && !root.open && root.activeFocus) {
            root._toggle()
            event.accepted = true
        }
    }

    Column {
        anchors.fill: parent
        spacing: 6

        Item {
            width: parent.width
            height: 46

            Column {
                anchors.left: parent.left
                anchors.right: chip.left
                anchors.rightMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2

                Txt {
                    text: root.label
                    color: Theme.fg
                    font.family: Theme.fontName
                    font.pixelSize: 13
                    font.bold: true
                    elide: Text.ElideRight
                }

                Txt {
                    visible: root.hint.length > 0
                    width: parent.width
                    wrapMode: Text.WordWrap
                    text: root.hint
                    color: Theme.fg3
                    font.family: Theme.fontName
                    font.pixelSize: 11
                }
            }

            Rectangle {
                id: chip
                width: Math.min(230, parent.width * 0.55)
                height: 40
                radius: 10
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                color: root.open ? Theme.acc : (chipMouse.containsMouse ? Theme.hover : Theme.bg2)
                border.width: 1
                border.color: root.open ? Theme.acc : Theme.bg3

                Behavior on color {
                    ColorAnimation { duration: 120 }
                }

                Txt {
                    anchors.left: parent.left
                    anchors.leftMargin: 12
                    anchors.right: chevronGlyph.left
                    anchors.rightMargin: 8
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.currentIndex >= 0 && root.currentIndex < root.entries.length
                        ? root.entries[root.currentIndex] : ""
                    color: root.open ? Theme.sfg : Theme.fg
                    font.family: Theme.fontName
                    font.pixelSize: 12
                    elide: Text.ElideRight
                }

                Txt {
                    id: chevronGlyph
                    anchors.right: parent.right
                    anchors.rightMargin: 10
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.open ? "\uf0d8" : "\uf0d7"
                    font.family: Theme.iconFont
                    font.pixelSize: 11
                    color: root.open ? Theme.sfg : Theme.fg3
                }

                MouseArea {
                    id: chipMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root._toggle()
                }
            }
        }

        Item {
            width: parent.width
            height: root.open ? root._listHeight : 0
            visible: root.open
            clip: true

            Rectangle {
                anchors.fill: parent
                radius: 10
                color: Theme.bg2
                border.width: 1
                border.color: Theme.bg3
            }

            Flickable {
                id: listFlick
                anchors.fill: parent
                anchors.margins: 1
                clip: true
                contentHeight: listView.contentHeight

                ListView {
                    id: listView
                    anchors.fill: parent
                    model: root.entries
                    interactive: false
                    boundsBehavior: Flickable.StopAtBounds

                    delegate: Item {
                        required property int index
                        required property string modelData

                        width: listView.width
                        height: 34

                        Rectangle {
                            anchors.fill: parent
                            radius: 7
                            color: index === root._selIndex ? Theme.acc
                                : (entryMouse.containsMouse ? Theme.hover : "transparent")

                            Behavior on color {
                                ColorAnimation { duration: 100 }
                            }
                        }

                        Txt {
                            anchors.left: parent.left
                            anchors.leftMargin: 12
                            anchors.right: parent.right
                            anchors.rightMargin: 8
                            anchors.verticalCenter: parent.verticalCenter
                            text: modelData
                            color: index === root._selIndex ? Theme.sfg : Theme.fg
                            font.family: Theme.fontName
                            font.pixelSize: 12
                            elide: Text.ElideRight
                        }

                        MouseArea {
                            id: entryMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onEntered: root._selIndex = index
                            onClicked: {
                                root._selIndex = index
                                root._commit()
                            }
                        }
                    }
                }
            }

            ScrollDragger {
                target: listFlick
                thumbWidth: 5
                minThumbHeight: 24
            }
        }
    }
}
