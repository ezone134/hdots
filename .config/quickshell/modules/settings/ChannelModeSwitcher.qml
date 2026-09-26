import QtQuick
import "../../components"
import "../../services"

// Channel / mode picker for the dotfiles settings. Mirrors what the
// channel_switcher script does: writes states2/s + states2/m
// (plus acc_changed so accent watchers re-fire), then re-applies the
// theme with theme_main restore. SettingSection listens for the resulting
// channel/mode property changes and reloads its channel-scoped rows.
Item {
    id: root

    width: parent ? parent.width : 0
    implicitHeight: col.implicitHeight

    readonly property var channels: [
        { key: "n", label: "Normal", icon: "\uf0c8" },
        { key: "d", label: "Dark", icon: "\uf186" },
        { key: "l", label: "Light", icon: "\uf185" }
    ]
    readonly property var modes: [
        { key: "d", label: "Dark", icon: "\uf186" },
        { key: "l", label: "Light", icon: "\uf185" }
    ]

    property bool _busy: false

    function setChannelMode(ch, mo) {
        if (root._busy)
            return
        root._busy = true
        SystemSettingsManager.writeFiles([
            { path: SystemSettingsManager.states2 + "/channel", value: ch },
            { path: SystemSettingsManager.states2 + "/mode", value: mo },
            { path: SystemSettingsManager.states2 + "/acc_changed", value: "1" },
            { path: SystemSettingsManager.states2 + "/waybar_restart_external", value: "1" }
        ], () => {
            // Mirror the channel_switcher script: notify the dotfiles'
            // Hyprland watchers that the channel/theme changed, then
            // regenerate the configs — one ordered bash run.
            SystemSettingsManager.run("module_reload sig_channel; module_reload sig_theme; theme_main restore")
            SystemSettingsManager.refreshChannel()
            root._busy = false
        })
    }

    Column {
        id: col
        anchors.left: parent.left
        anchors.right: parent.right
        spacing: 8

        // ---- Channel row ----
        Row {
            width: parent.width
            spacing: 8

            Txt {
                width: 52
                anchors.verticalCenter: parent.verticalCenter
                text: "Channel"
                color: Theme.fg3
                font.family: Theme.fontName
                font.pixelSize: 11
                font.bold: true
            }

            Repeater {
                model: root.channels

                delegate: Rectangle {
                    required property var modelData

                    property bool active: modelData.key === SystemSettingsManager.channel

                    width: 88
                    height: 34
                    radius: 10
                    color: active ? Theme.acc : (pillMouse.containsMouse ? Theme.hover : Theme.bg2)
                    border.width: 1
                    border.color: active ? Theme.acc : Theme.bg3
                    opacity: root._busy ? 0.6 : 1

                    Behavior on color {
                        ColorAnimation { duration: 120 }
                    }
                    Behavior on opacity {
                        NumberAnimation { duration: 120 }
                    }

                    Row {
                        anchors.centerIn: parent
                        spacing: 6

                        Txt {
                            anchors.verticalCenter: parent.verticalCenter
                            text: modelData.icon
                            font.family: Theme.iconFont
                            font.pixelSize: 12
                            color: active ? Theme.sfg : Theme.fg
                        }

                        Txt {
                            anchors.verticalCenter: parent.verticalCenter
                            text: modelData.label
                            color: active ? Theme.sfg : Theme.fg
                            font.family: Theme.fontName
                            font.pixelSize: 12
                            font.bold: active
                        }
                    }

                    MouseArea {
                        id: pillMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.setChannelMode(modelData.key, SystemSettingsManager.mode)
                    }
                }
            }
        }

        // ---- Mode row ----
        Row {
            width: parent.width
            spacing: 8

            Txt {
                width: 52
                anchors.verticalCenter: parent.verticalCenter
                text: "Mode"
                color: Theme.fg3
                font.family: Theme.fontName
                font.pixelSize: 11
                font.bold: true
            }

            Repeater {
                model: root.modes

                delegate: Rectangle {
                    required property var modelData

                    property bool active: modelData.key === SystemSettingsManager.mode

                    width: 88
                    height: 34
                    radius: 10
                    color: active ? Theme.acc : (pillMouse.containsMouse ? Theme.hover : Theme.bg2)
                    border.width: 1
                    border.color: active ? Theme.acc : Theme.bg3
                    opacity: root._busy ? 0.6 : 1

                    Behavior on color {
                        ColorAnimation { duration: 120 }
                    }
                    Behavior on opacity {
                        NumberAnimation { duration: 120 }
                    }

                    Row {
                        anchors.centerIn: parent
                        spacing: 6

                        Txt {
                            anchors.verticalCenter: parent.verticalCenter
                            text: modelData.icon
                            font.family: Theme.iconFont
                            font.pixelSize: 12
                            color: active ? Theme.sfg : Theme.fg
                        }

                        Txt {
                            anchors.verticalCenter: parent.verticalCenter
                            text: modelData.label
                            color: active ? Theme.sfg : Theme.fg
                            font.family: Theme.fontName
                            font.pixelSize: 12
                            font.bold: active
                        }
                    }

                    MouseArea {
                        id: pillMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.setChannelMode(SystemSettingsManager.channel, modelData.key)
                    }
                }
            }
        }
    }
}
