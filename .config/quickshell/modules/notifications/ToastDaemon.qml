pragma ComponentBehavior: Bound
import QtQuick
import Quickshell
import Quickshell.Wayland
import "."
import "../../components"
import "../../services"

// Transient toast notifications. This daemon renders the live toast list
// from NotificationManager as corner popups that slide in from the screen
// edge, with action buttons, inline reply and hover-to-pause auto-dismiss.
// It replaces the mako-style NotificationDaemon entirely.
Scope {
    id: root

    readonly property string toastAnchor: (SettingsManager.config.notifications && SettingsManager.config.notifications.anchor) || "top-right"
    readonly property int toastWidth: (SettingsManager.config.notifications && SettingsManager.config.notifications.width) || 300
    readonly property int margin: 10
    readonly property int spacing: 8

    // Track open inline-reply fields so the toast window only asks for
    // keyboard focus while replying. With focusable+OnDemand up all the
    // time, the overlay surface can steal and hold keyboard focus from the
    // focused window (Quickshell warns OnDemand may retain focus "over
    // another window unexpectedly"), swallowing all key input until the
    // toast dismisses.
    property var replyingCards: []
    function setReplying(card, replying) {
        const idx = root.replyingCards.indexOf(card)
        if (replying && idx === -1)
            root.replyingCards = root.replyingCards.concat([card])
        else if (!replying && idx !== -1) {
            const arr = root.replyingCards.slice()
            arr.splice(idx, 1)
            root.replyingCards = arr
        }
    }
    readonly property bool anyReplying: root.replyingCards.length > 0

    function isTop()  { return root.toastAnchor === "top-right" || root.toastAnchor === "top-left" }
    function isLeft() { return root.toastAnchor === "top-left" || root.toastAnchor === "bottom-left" }

    // Slide-in/out offsets: toasts enter from the nearest screen edge.
    function slideX() {
        if (root.isLeft())
            return -(root.toastWidth + 24)
        return root.toastWidth + 24
    }
    function slideY() {
        if (root.isTop())
            return -96
        return 96
    }

    function resolveIcon(icon) {
        const i = String(icon || "").trim()
        if (i.length === 0)
            return ""
        if (i.indexOf("/") === 0)
            return "file://" + i
        return ""
    }

    function resolveImage(path) {
        const p = String(path || "").trim()
        if (p.length === 0)
            return ""
        if (p.indexOf("/") === 0)
            return "file://" + p
        return ""
    }

    function isImage(path) {
        const p = String(path || "").trim()
        return p.length > 0 && p.indexOf("/") === 0
    }

    // One layer-shell window per screen, repositioned to the configured
    // corner. (Previously four PanelWindows per screen were always alive —
    // a 4x surface/object tax that shares the render thread with the pill
    // morph, which is why opening a widget while a toast was up stuttered.)
    Variants {
        model: Quickshell.screens

        delegate: Scope {
            id: perScreen
            required property var modelData

            PanelWindow {
                visible: toastRepeater.count > 0
                screen: perScreen.modelData
                color: "transparent"
                focusable: root.anyReplying
                anchors {
                    top: root.isTop()
                    bottom: !root.isTop()
                    left: root.isLeft()
                    right: !root.isLeft()
                }
                implicitWidth: root.toastWidth + root.margin
                implicitHeight: toastCol.implicitHeight + root.margin
                WlrLayershell.layer: WlrLayer.Overlay
                WlrLayershell.keyboardFocus: root.anyReplying ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None

                Rectangle {
                    anchors.fill: parent
                    color: "transparent"
                    clip: true

                    Column {
                        id: toastCol
                        anchors.top: root.isTop() ? parent.top : undefined
                        anchors.bottom: !root.isTop() ? parent.bottom : undefined
                        anchors.left: root.isLeft() ? parent.left : undefined
                        anchors.right: !root.isLeft() ? parent.right : undefined
                        anchors.topMargin: root.margin
                        anchors.bottomMargin: root.margin
                        anchors.leftMargin: root.margin
                        anchors.rightMargin: root.margin
                        width: root.toastWidth
                        spacing: root.spacing

                        Repeater {
                            id: toastRepeater
                            model: NotificationManager.notifications
                            delegate: toastCard
                        }
                    }
                }
            }
        }
    }

    Component {
        id: toastCard

        Rectangle {
            id: card
            required property var modelData

            readonly property bool hasActions: (card.modelData.actions && card.modelData.actions.length > 0)
            readonly property int totalMs: card.modelData.timeoutMs > 0 ? card.modelData.timeoutMs : 0
            property bool hovered: false
            property bool replying: false
            property bool removing: false

            // Resume-aware auto-dismiss. `deadline` is when the toast should
            // go; hovering freezes it (capturing the remaining time in
            // `holdMs`) and leaving resumes from that remaining time instead
            // of restarting the full duration. `tickMs` drives the single-
            // shot timer's interval (never assign Timer.interval from JS —
            // that throws in Quickshell 0.3.x, so the timer binds to it).
            property double deadline: 0
            property int holdMs: 0
            property int tickMs: card.totalMs

            width: root.toastWidth
            implicitHeight: contentCol.implicitHeight + 24
            radius: 14
            color: Theme.bg
            border.width: 1
            border.color: card.hovered ? Theme.acc : Theme.border
            clip: true
            transform: Translate { id: slide }

            function enter() {
                slide.x = root.slideX()
                slide.y = root.slideY()
                card.opacity = 0
                enterAnim.restart()
            }

            function dismiss() {
                if (card.removing)
                    return
                card.removing = true
                dismissTimer.stop()
                exitAnim.start()
            }

            function runAction(action) {
                if (action && action.invoke) {
                    try { action.invoke() } catch (e) {}
                }
                card.dismiss()
            }

            function sendReply() {
                const text = replyField.text.trim()
                if (card.modelData.notification && card.modelData.hasInlineReply) {
                    try { card.modelData.notification.sendInlineReply(text) } catch (e) {}
                }
                card.dismiss()
            }

            ParallelAnimation {
                id: enterAnim
                running: false
                NumberAnimation { target: slide; property: "x"; to: 0; duration: 260; easing.type: Easing.OutCubic }
                NumberAnimation { target: slide; property: "y"; to: 0; duration: 260; easing.type: Easing.OutCubic }
                NumberAnimation { target: card; property: "opacity"; to: 1; duration: 260; easing.type: Easing.OutCubic }
            }

            SequentialAnimation {
                id: exitAnim
                running: false

                ParallelAnimation {
                    NumberAnimation { target: slide; property: "x"; to: root.slideX(); duration: 200; easing.type: Easing.InCubic }
                    NumberAnimation { target: slide; property: "y"; to: root.slideY(); duration: 200; easing.type: Easing.InCubic }
                    NumberAnimation { target: card; property: "opacity"; to: 0; duration: 200; easing.type: Easing.InCubic }
                }

                ScriptAction {
                    script: NotificationManager.removeNotification(card.modelData.id)
                }
            }

            // Auto-dismiss: ONE single-shot timer. No countdown bar and no
            // periodic ticking — a toast visible on screen burns zero CPU
            // while waiting (the timer just sleeps, then fires once).
            // Hovering stops the timer and captures the remaining time;
            // leaving resumes from it. This replaced the old 250ms repeat
            // ticker whose progress-bar relayout was the main-thread churn
            // source.
            Timer {
                id: dismissTimer
                interval: card.tickMs
                running: card.totalMs > 0 && !card.hovered && !card.replying && !card.removing
                onTriggered: card.dismiss()
            }

            // Shared pause/resume: hovering OR opening the inline reply
            // freezes the countdown (capturing the remaining time); leaving
            // either resumes from it. The deadline only moves on resume, so
            // overlapping pauses (hover while replying) stay consistent.
            function pauseHold() {
                card.holdMs = Math.max(0, card.deadline - Date.now())
            }

            function resumeHold() {
                if (card.totalMs > 0) {
                    card.deadline = Date.now() + card.holdMs
                    card.tickMs = Math.max(1, card.holdMs)
                    dismissTimer.start()
                }
            }

            onHoveredChanged: {
                if (card.hovered)
                    card.pauseHold()
                else
                    card.resumeHold()
            }

            onReplyingChanged: {
                root.setReplying(card, card.replying)
                if (card.replying)
                    card.pauseHold()
                else
                    card.resumeHold()
            }

            Component.onCompleted: {
                if (card.totalMs > 0)
                    card.deadline = Date.now() + card.totalMs
                card.enter()
            }

            // Background: click to dismiss, hover to pause the timer.
            MouseArea {
                anchors.fill: parent
                hoverEnabled: true
                acceptedButtons: Qt.LeftButton
                onEntered: card.hovered = true
                onExited: card.hovered = false
                onClicked: card.dismiss()
            }

            Column {
                id: contentCol
                anchors.fill: parent
                anchors.margins: 12
                spacing: 8

                // Header: app icon, app name, close button
                Row {
                    width: parent.width
                    spacing: 10

                    Item {
                        width: 36
                        height: 36

                        Image {
                            anchors.fill: parent
                            source: root.resolveIcon(card.modelData.appIcon)
                            visible: source.length > 0
                            fillMode: Image.PreserveAspectFit
                            asynchronous: true
                        }

                        Rectangle {
                            anchors.fill: parent
                            radius: 10
                            color: Theme.bg2
                            visible: root.resolveIcon(card.modelData.appIcon).length === 0

                            Txt {
                                anchors.centerIn: parent
                                text: "\uf2d0"
                                font.family: Theme.fontName
                                font.pixelSize: 18
                                color: Theme.fg2
                            }
                        }
                    }

                    Txt {
                        width: parent.width - 36 - 10 - 24 - 10
                        anchors.verticalCenter: parent.verticalCenter
                        text: card.modelData.appName.length > 0 ? card.modelData.appName : "Notification"
                        color: Theme.fg2
                        font.family: Theme.fontName
                        font.pixelSize: 11
                        elide: Text.ElideRight
                    }

                    Rectangle {
                        width: 24
                        height: 24
                        radius: 7
                        color: "transparent"

                        Txt {
                            anchors.centerIn: parent
                            text: "\uf00d"
                            font.family: Theme.fontName
                            font.pixelSize: 11
                            color: Theme.fg3
                        }

                        MouseArea {
                            anchors.fill: parent
                            hoverEnabled: true
                            onEntered: { parent.color = Theme.bg3; card.hovered = true }
                            onExited: { parent.color = "transparent"; card.hovered = false }
                            onClicked: card.dismiss()
                        }
                    }
                }

                // Title
                Txt {
                    width: parent.width
                    text: card.modelData.summary
                    color: Theme.fg
                    font.family: Theme.fontName
                    font.pixelSize: 14
                    font.bold: true
                    wrapMode: Text.Wrap
                }

                // Body
                Txt {
                    visible: card.modelData.body.length > 0
                    width: parent.width
                    text: card.modelData.body
                    color: Theme.fg2
                    font.family: Theme.fontName
                    font.pixelSize: 12
                    wrapMode: Text.Wrap
                }

                // Notification image (e.g. chat avatar)
                Image {
                    visible: root.isImage(card.modelData.image)
                    source: root.resolveImage(card.modelData.image)
                    width: Math.min(parent.width, 160)
                    height: Math.min(parent.width, 160)
                    anchors.horizontalCenter: parent.horizontalCenter
                    fillMode: Image.PreserveAspectFit
                    asynchronous: true
                }

                // Action buttons + inline reply toggle
                Flow {
                    visible: card.hasActions || card.modelData.hasInlineReply
                    width: parent.width
                    spacing: 8

                    Repeater {
                        model: card.modelData.actions || []

                        delegate: Rectangle {
                            required property var modelData
                            id: actionBtn

                            width: actionLabel.implicitWidth + 20
                            height: 28
                            radius: 8
                            color: Theme.bg2

                            Txt {
                                id: actionLabel
                                anchors.centerIn: parent
                                text: modelData.text
                                color: Theme.fg
                                font.family: Theme.fontName
                                font.pixelSize: 12
                            }

                            MouseArea {
                                anchors.fill: parent
                                hoverEnabled: true
                                onEntered: { actionBtn.color = Theme.bg3; card.hovered = true }
                                onExited: { actionBtn.color = Theme.bg2; card.hovered = false }
                                onClicked: card.runAction(modelData)
                            }
                        }
                    }

                    Rectangle {
                        visible: card.modelData.hasInlineReply
                        width: replyLabel.implicitWidth + 20
                        height: 28
                        radius: 8
                        color: card.replying ? Theme.acc : Theme.bg2

                        Txt {
                            id: replyLabel
                            anchors.centerIn: parent
                            text: card.replying ? "\uf00d" : "\uf086"
                            font.family: Theme.fontName
                            font.pixelSize: 12
                            color: card.replying ? Theme.sfg : Theme.fg
                        }

                        MouseArea {
                            anchors.fill: parent
                            hoverEnabled: true
                            onEntered: { card.hovered = true }
                            onExited: { card.hovered = false }
                            onClicked: {
                                card.replying = !card.replying
                                if (card.replying)
                                    replyField.forceActiveFocus()
                            }
                        }
                    }
                }

                // Inline reply input
                Row {
                    visible: card.replying && card.modelData.hasInlineReply
                    width: parent.width
                    spacing: 8

                    Rectangle {
                        width: parent.width - 40
                        height: 32
                        radius: 8
                        color: Theme.bg3

                        Txt {
                            visible: replyField.text.length === 0
                            anchors.left: parent.left
                            anchors.leftMargin: 10
                            anchors.right: parent.right
                            anchors.rightMargin: 10
                            anchors.verticalCenter: parent.verticalCenter
                            text: card.modelData.inlineReplyPlaceholder || "Reply"
                            color: Theme.fg3
                            font.family: Theme.fontName
                            font.pixelSize: 12
                            elide: Text.ElideRight
                        }

                        TxtInput {
                            id: replyField
                            anchors.fill: parent
                            anchors.leftMargin: 10
                            anchors.rightMargin: 10
                            verticalAlignment: Text.AlignVCenter
                            color: Theme.fg
                            font.family: Theme.fontName
                            font.pixelSize: 12

                            Keys.onPressed: function(event) {
                                if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                                    card.sendReply()
                                    event.accepted = true
                                } else if (event.key === Qt.Key_Escape) {
                                    card.replying = false
                                    event.accepted = true
                                }
                            }
                        }
                    }

                    Rectangle {
                        width: 32
                        height: 32
                        radius: 8
                        color: Theme.acc

                        Txt {
                            anchors.centerIn: parent
                            text: "\uf1d8"
                            font.family: Theme.fontName
                            font.pixelSize: 13
                            color: Theme.sfg
                        }

                        MouseArea {
                            anchors.fill: parent
                            hoverEnabled: true
                            onEntered: card.hovered = true
                            onExited: card.hovered = false
                            onClicked: card.sendReply()
                        }
                    }
                }

            }
        }
    }
}