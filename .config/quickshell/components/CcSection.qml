import QtQuick
import QtQuick.Layouts
import "."
import "../services"

// Rounded list well + optional header row (Wi‑Fi / Bluetooth style).
Item {
    id: root

    property string title: ""
    property string iconType: ""
    property string trailing: ""
    property bool scanning: false
    property alias listHeight: well.height
    property alias contentItem: listHost

    signal refreshClicked()

    implicitWidth: 280
    implicitHeight: headerRow.height + 8 + well.height

    Column {
        anchors.fill: parent
        spacing: 8

        RowLayout {
            id: headerRow
            width: parent.width
            spacing: 8

            StatusIcon {
                visible: root.iconType.length > 0
                Layout.alignment: Qt.AlignVCenter
                type: root.iconType
                active: true
                iconSize: 14
                color: Theme.fg2
            }

            Txt {
                Layout.alignment: Qt.AlignVCenter
                text: root.title
                color: Theme.fg2
                font.family: Theme.fontName
                font.pixelSize: 12
                font.bold: true
                font.letterSpacing: 0.3
            }

            Txt {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                visible: root.trailing.length > 0
                text: root.trailing
                color: Theme.acc
                font.family: Theme.fontName
                font.pixelSize: 11
                elide: Text.ElideRight
                horizontalAlignment: Text.AlignRight
            }

            Item {
                Layout.preferredWidth: 28
                Layout.preferredHeight: 28
                Layout.alignment: Qt.AlignVCenter

                Rectangle {
                    anchors.fill: parent
                    radius: 14
                    color: root.scanning ? Theme.acc : Theme.bg3
                    Behavior on color {
                        ColorAnimation { duration: 150 }
                    }

                    StatusIcon {
                        anchors.centerIn: parent
                        type: "refresh"
                        iconSize: 13
                        color: root.scanning ? Theme.sfg : Theme.fg2
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.refreshClicked()
                    }
                }
            }
        }

        Rectangle {
            id: well
            width: parent.width
            height: 200
            radius: 16
            color: Theme.bg3
            border.width: 1
            border.color: Theme.border
            clip: true

            // Children of CcSection are reparented here via default property
            Item {
                id: listHost
                anchors.fill: parent
                anchors.margins: 4
            }
        }
    }

    default property alias data: listHost.data
}
