pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import "."
import "../../components"
import "../../services"

Item {
    id: root
    implicitWidth: 720
    implicitHeight: 190
    focus: true

    Component.onCompleted: {
        console.log("[power] panel opened")
        forceActiveFocus()
    }
    onVisibleChanged: {
        if (visible)
            forceActiveFocus()
    }

    property string armedAction: ""
    property string armedCommand: ""
    property real holdProgress: 0
    property int holdDuration: 900
    property int selectedIndex: 0
    property real contentOpacity: 0
    property real contentScale: 0.98

    property var actions: [
        { key: "lock", icon: "󰌾", label: "Lock", subtitle: "Secure this session", command: "power_main session lock" },
        { key: "logout", icon: "󰍃", label: "Logout", subtitle: "Exit Hyprland", command: "power_main session logout" },
        { key: "suspend", icon: "󰤄", label: "Sleep", subtitle: "Suspend to RAM", command: "power_main session suspend" },
        { key: "hibernate", icon: "󰤆", label: "Hibernate", subtitle: "Suspend to disk", command: "power_main session hibernate" },
        { key: "reboot", icon: "󰜉", label: "Reboot", subtitle: "Restart the system", command: "power_main session reboot" },
        { key: "shutdown", icon: "󰐥", label: "Power Off", subtitle: "Turn everything off", command: "power_main session shutdown" },
        { key: "cancel", icon: "󰅖", label: "Cancel", subtitle: "Close the power menu", command: "" }
    ]

    function actionAccent(key) {
        if (key === "reboot" || key === "shutdown")
            return "#8B0000ff"
        if (key === "hibernate")
            return "#8B0000ff"
        if (key === "cancel")
            return Theme.fg2
        return Theme.acc
    }

    function currentAction() {
        return root.actions[root.selectedIndex] || root.actions[0]
    }

    function isDanger(key) {
        return key === "reboot" || key === "shutdown" || key === "hibernate"
    }

    function isCancel(key) {
        return key === "cancel"
    }

    function selectIndex(index) {
        if (index < 0)
            index = 0
        if (index >= root.actions.length)
            index = root.actions.length - 1
        if (index === root.selectedIndex)
            return
        root.selectedIndex = index
        root.cancelHold()
    }

    function beginHold(action) {
        if (!action)
            return
        if (root.isCancel(action.key)) {
            root.cancelHold()
            StateController.pillReset()
            return
        }
        root.armedAction = action.key
        root.armedCommand = action.command
        root.holdProgress = 0
        console.log("[power] hold started: " + action.key + " -> " + action.command)
        holdAnim.restart()
    }

    function triggerCurrentHold() {
        root.beginHold(root.currentAction())
    }

    function cancelHold() {
        if (root.armedAction.length > 0)
            console.log("[power] hold cancelled: " + root.armedAction)
        holdAnim.stop()
        root.holdProgress = 0
        root.armedAction = ""
    }

    Keys.onPressed: function(event) {
        if (event.key === Qt.Key_Escape) {
            console.log("[power] closed (Esc)")
            root.cancelHold()
            StateController.pillReset()
            event.accepted = true
            return
        }

        if (event.key === Qt.Key_Left || event.key === Qt.Key_Up || (event.key === Qt.Key_Tab && (event.modifiers & Qt.ShiftModifier))) {
            root.selectIndex(root.selectedIndex - 1)
            event.accepted = true
            return
        }

        if (event.key === Qt.Key_Right || event.key === Qt.Key_Down || (event.key === Qt.Key_Tab && !(event.modifiers & Qt.ShiftModifier))) {
            root.selectIndex(root.selectedIndex + 1)
            event.accepted = true
            return
        }

        if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Space) {
            if (!event.isAutoRepeat && root.armedAction !== root.currentAction().key)
                root.triggerCurrentHold()
            event.accepted = true
        }
    }

    Keys.onReleased: function(event) {
        if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Space) {
            if (holdAnim.running)
                root.cancelHold()
            event.accepted = true
        }
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
            anchors.margins: 16
            spacing: 12

            Txt {
                anchors.horizontalCenter: parent.horizontalCenter
                text: root.armedAction.length > 0 ? "Confirm " + root.currentAction().label : "Power"
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
                    spacing: 14

                    RowLayout {
                        width: parent.width
                        spacing: 10

                        Repeater {
                            model: root.actions

                            delegate: Item {
                                id: actionItem
                                required property var modelData
                                required property int index
                                Layout.fillWidth: true
                                Layout.preferredHeight: 102

                                property bool selected: root.selectedIndex === index
                                property bool armed: root.armedAction === modelData.key
                                property color accentColor: root.actionAccent(modelData.key)

                                Rectangle {
                                    anchors.fill: parent
                                    radius: 20
                                    color: actionItem.armed
                                        ? actionItem.accentColor
                                        : actionItem.selected
                                          ? Qt.rgba(0, 0, 0, 0.10)
                                          : Qt.rgba(0, 0, 0, 0.02)
                                    border.width: actionItem.selected || actionItem.armed ? 2 : 1
                                    border.color: actionItem.armed
                                        ? actionItem.accentColor
                                        : actionItem.selected
                                          ? actionItem.accentColor
                                          : Qt.rgba(1, 1, 1, 0.35)

                                    Behavior on color {
                                        ColorAnimation { duration: 150 }
                                    }

                                    Behavior on border.color {
                                        ColorAnimation { duration: 150 }
                                    }

                                    Column {
                                        anchors.centerIn: parent
                                        spacing: 7

                                        Rectangle {
                                            width: 42
                                            height: 42
                                            radius: 21
                                            anchors.horizontalCenter: parent.horizontalCenter
                                            color: Theme.bg
                                            border.width: actionItem.selected && !actionItem.armed ? 1 : 0
                                            border.color: actionItem.accentColor

                                            Txt {
                                                anchors.centerIn: parent
                                                text: actionItem.modelData.icon
                                                font.family: Theme.iconFont
                                                font.pixelSize: 20
                                                color: actionItem.armed ? actionItem.accentColor : Theme.fg
                                            }
                                        }

                                        Txt {
                                            anchors.horizontalCenter: parent.horizontalCenter
                                            text: actionItem.modelData.label
                                            font.family: Theme.fontName
                                            font.pixelSize: 11
                                            font.bold: true
                                            color: actionItem.armed
                                                ? (root.isDanger(actionItem.modelData.key) ? Theme.bg : Theme.sfg)
                                                : Theme.fg
                                        }
                                    }

                                    Rectangle {
                                        visible: actionItem.selected && !actionItem.armed
                                        anchors.top: parent.top
                                        anchors.horizontalCenter: parent.horizontalCenter
                                        anchors.topMargin: 7
                                        width: 26
                                        height: 4
                                        radius: 2
                                        color: actionItem.accentColor
                                    }

                                    Rectangle {
                                        anchors.left: parent.left
                                        anchors.right: parent.right
                                        anchors.bottom: parent.bottom
                                        anchors.margins: 8
                                        height: 4
                                        radius: 2
                                        color: Qt.rgba(1, 1, 1, actionItem.armed ? 0.28 : 0)

                                        Rectangle {
                                            anchors.left: parent.left
                                            anchors.top: parent.top
                                            anchors.bottom: parent.bottom
                                            width: actionItem.armed ? parent.width * root.holdProgress : 0
                                            radius: 2
                                            color: actionItem.armed
                                                ? (root.isDanger(actionItem.modelData.key) ? Theme.bg : Theme.sfg)
                                                : actionItem.accentColor
                                        }
                                    }

                                    MouseArea {
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        onEntered: root.selectIndex(actionItem.index)
                                        onPressed: function(mouse) {
                                            mouse.accepted = true
                                            root.selectedIndex = actionItem.index
                                            root.beginHold(actionItem.modelData)
                                        }
                                        onReleased: function(mouse) {
                                            mouse.accepted = true
                                            if (holdAnim.running)
                                                root.cancelHold()
                                        }
                                        onCanceled: root.cancelHold()
                                    }
                                }
                            }
                        }
                    }

                    Row {
                        anchors.horizontalCenter: parent.horizontalCenter
                        spacing: 14

                        Txt {
                            text: "← → select"
                            color: Theme.fg3
                            font.family: Theme.fontName
                            font.pixelSize: 12
                        }

                        Txt {
                            text: "hold Enter or click"
                            color: Theme.fg3
                            font.family: Theme.fontName
                            font.pixelSize: 12
                        }

                        Txt {
                            text: "Esc cancel"
                            color: Theme.fg3
                            font.family: Theme.fontName
                            font.pixelSize: 12
                        }
                    }
                }
            }
        }
    }

    SequentialAnimation {
        id: holdAnim
        running: false

        PropertyAnimation {
            target: root
            property: "holdProgress"
            from: 0
            to: 1
            duration: root.holdDuration
        }

        ScriptAction {
            script: {
                if (root.armedAction.length > 0) {
                    console.log("[power] hold complete — firing: " + root.armedAction)
                    // Runs on StateController's long-lived Process: the panel
                    // is destroyed by pillReset right after, but the command
                    // keeps running and its exit/stderr report back.
                    StateController.runPowerAction(root.armedCommand)
                    root.armedAction = ""
                    root.cancelHold()
                    StateController.pillReset()
                }
            }
        }
    }
}