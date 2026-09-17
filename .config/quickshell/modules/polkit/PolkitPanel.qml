import QtQuick
import QtQuick.Layouts
import "."
import "../../components"
import "../../services"

// Auth dialog that lives inside the morph surface: the bar grows into
// this island whenever a polkit request is active.
Item {
    id: root
    implicitWidth: 460
    implicitHeight: contentColumn.implicitHeight + 32
    focus: true

    property var flow: PolkitManager.flow
    property bool responseRequired: root.flow && root.flow.isResponseRequired
    property bool failed: root.flow && root.flow.failed

    onResponseRequiredChanged: {
        if (root.responseRequired) {
            passwordInput.text = ""
            passwordInput.forceActiveFocus()
        }
    }

    onFailedChanged: {
        if (root.failed) {
            passwordInput.text = ""
            passwordInput.forceActiveFocus()
        }
    }

    onFlowChanged: {
        passwordInput.text = ""
        if (root.responseRequired)
            passwordInput.forceActiveFocus()
    }

    function submitAuth() {
        if (!root.flow)
            return
        PolkitManager.submit(passwordInput.text)
        passwordInput.text = ""
        if (root.flow.isResponseRequired)
            passwordInput.forceActiveFocus()
    }

    function cancelAuth() {
        if (!root.flow)
            return
        PolkitManager.cancel()
        passwordInput.text = ""
    }

    ColumnLayout {
        id: contentColumn
        anchors.fill: parent
        anchors.margins: 16
        spacing: 10

        RowLayout {
            Layout.fillWidth: true
            spacing: 12

            Rectangle {
                Layout.preferredWidth: 44
                Layout.preferredHeight: 44
                radius: 22
                color: Theme.bg3

                Txt {
                    anchors.centerIn: parent
                    text: "󰌾"
                    font.family: Theme.iconFont
                    font.pixelSize: 22
                    color: Theme.fg
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Txt {
                    Layout.fillWidth: true
                    text: "Authentication Required"
                    color: Theme.fg
                    font.family: Theme.fontName
                    font.pixelSize: 18
                    font.bold: true
                }

                Txt {
                    Layout.fillWidth: true
                    text: root.flow ? root.flow.actionId : ""
                    color: Theme.fg3
                    font.family: Theme.fontName
                    font.pixelSize: 11
                    elide: Text.ElideRight
                }
            }
        }

        Txt {
            Layout.fillWidth: true
            visible: root.flow && root.flow.message.length > 0
            text: root.flow ? root.flow.message : ""
            wrapMode: Text.WordWrap
            color: Theme.fg
            font.family: Theme.fontName
            font.pixelSize: 14
            font.bold: true
        }

        Txt {
            Layout.fillWidth: true
            visible: root.flow && root.flow.supplementaryMessage.length > 0
            text: root.flow ? root.flow.supplementaryMessage : ""
            wrapMode: Text.WordWrap
            color: root.flow && root.flow.supplementaryIsError ? "#8B0000ff" : Theme.fg3
            font.family: Theme.fontName
            font.pixelSize: 12
        }

        Txt {
            Layout.fillWidth: true
            visible: root.failed
            text: "Authentication failed. Try again."
            color: "#8B0000ff"
            font.family: Theme.fontName
            font.pixelSize: 12
            font.bold: true
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 44
            radius: 16
            color: Qt.rgba(0, 0, 0, 0.06)
            border.width: 1
            border.color: passwordInput.activeFocus ? Theme.acc : Qt.rgba(1, 1, 1, 0.35)

            TxtInput {
                id: passwordInput
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 14
                color: Theme.fg
                font.family: Theme.fontName
                font.pixelSize: 14
                echoMode: root.flow && root.flow.responseVisible ? TextInput.Normal : TextInput.Password
                passwordCharacter: "•"
                selectByMouse: true
                clip: true

                onAccepted: submitAuth()
            }

            Txt {
                anchors.left: parent.left
                anchors.leftMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                visible: passwordInput.text.length === 0 && !passwordInput.activeFocus
                text: root.flow ? root.flow.inputPrompt : "Password"
                color: Theme.fg3
                font.family: Theme.fontName
                font.pixelSize: 13
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 38
                radius: 19
                color: Qt.rgba(0, 0, 0, 0.06)
                border.width: 1
                border.color: Qt.rgba(1, 1, 1, 0.35)

                Txt {
                    anchors.centerIn: parent
                    text: "Cancel"
                    color: Theme.fg
                    font.family: Theme.fontName
                    font.pixelSize: 13
                    font.bold: true
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: cancelAuth()
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 38
                radius: 19
                color: Theme.acc

                Txt {
                    anchors.centerIn: parent
                    text: "Authenticate"
                    color: Theme.sfg
                    font.family: Theme.fontName
                    font.pixelSize: 13
                    font.bold: true
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: submitAuth()
                }
            }
        }
    }

    Keys.onPressed: function(event) {
        if (event.key === Qt.Key_Escape) {
            cancelAuth()
            event.accepted = true
        } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            submitAuth()
            event.accepted = true
        }
    }

    Component.onCompleted: {
        if (root.responseRequired)
            passwordInput.forceActiveFocus()
    }
}