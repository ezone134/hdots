import QtQuick
import QtQuick.Layouts
import "."
import "../../components"
import "../../services"

// Audio routing panel: pick the active output (sink) and input (source), or
// control per-app stream volumes (sink-inputs). Lists are fetched on open via
// AudioManager.refresh(); selecting a device sets the PipeWire default and
// the list reloads. No polling.
Item {
    id: root
    implicitWidth: 520
    implicitHeight: 400
    focus: true

    property int selTab: 0          // 0 = outputs, 1 = inputs, 2 = streams, 3 = record
    property int selIndex: 0
    property string recorderPane: "" // "" = main, "recordings" = 2nd pane
    property int recSelIndex: 0
    property string recName: ""

    // In-panel OSD chip: flashes the stream app + new level while adjusting
    // (the island toast would collapse this modal panel, so feedback stays
    // local). One-shot fade — zero CPU while idle.
    property string osdText: ""
    property real osdOpacity: 0

    Timer {
        id: osdTimer
        interval: 900
        repeat: false
        onTriggered: root.osdOpacity = 0
    }

    function showOsd(text) {
        root.osdText = text
        root.osdOpacity = 1
        osdTimer.restart()
    }

    property var listModel: selTab === 0 ? AudioManager.sinks
        : selTab === 1 ? AudioManager.sources
        : selTab === 2 ? AudioManager.streams
        : []

    Component.onCompleted: {
        AudioManager.refresh()
    }

    onVisibleChanged: {
        if (visible) {
            AudioManager.refresh()
        }
    }

    OverlayFocusScope {
        focusTarget: root
    }

    function clamp(i, len) {
        if (len === 0)
            return 0
        if (i < 0)
            return len - 1
        if (i >= len)
            return 0
        return i
    }

    function switchTab(delta) {
        root.selTab = (root.selTab + delta + 4) % 4
        root.selIndex = 0
    }

    function moveSel(delta) {
        root.selIndex = root.clamp(root.selIndex + delta, root.listModel.length)
    }

    function activate() {
        const items = root.listModel
        if (items.length === 0 || root.selIndex >= items.length)
            return
        const device = items[root.selIndex]
        if (root.selTab === 0)
            AudioManager.setSink(device.name)
        else if (root.selTab === 1)
            AudioManager.setSource(device.name)
        StateController.resetHideTimer()
    }

    // ---- per-app stream controls ----------------------------------------

    function adjustVol(delta) {
        const items = AudioManager.streams
        if (items.length === 0 || root.selIndex >= items.length)
            return
        const s = items[root.selIndex]
        const target = Math.max(0, Math.min(150, s.vol + delta))
        AudioManager.setStreamVolume(s.id, target)
        root.showOsd(s.app + "  " + target + "%")
    }

    function toggleMute() {
        const items = AudioManager.streams
        if (items.length === 0 || root.selIndex >= items.length)
            return
        const s = items[root.selIndex]
        AudioManager.setStreamMute(s.id)
        root.showOsd(s.app + (s.muted ? "  unmuted" : "  muted"))
    }

    function volBarWidth(listWidth) {
        const items = AudioManager.streams
        if (items.length === 0 || root.selIndex >= items.length)
            return 0
        return Math.max(2, Math.min(1, items[root.selIndex].vol / 100) * listWidth)
    }

    Keys.onPressed: function(event) {
        if (event.key === Qt.Key_Escape) {
            if (root.recorderPane === "recordings") {
                root.recorderPane = ""
                event.accepted = true
            } else {
                StateController.pillReset()
                event.accepted = true
            }
        } else if (event.key === Qt.Key_Tab) {
            root.switchTab(event.modifiers & Qt.ShiftModifier ? -1 : 1)
            event.accepted = true
        } else if (event.key === Qt.Key_Up) {
            if (root.selTab === 3) {
                root.recSelIndex = Math.max(0, root.recSelIndex - 1)
            } else {
                root.moveSel(-1)
            }
            event.accepted = true
        } else if (event.key === Qt.Key_Down) {
            if (root.selTab === 3) {
                root.recSelIndex = Math.min(AudioRecorderManager.recordings.length - 1, root.recSelIndex + 1)
            } else {
                root.moveSel(1)
            }
            event.accepted = true
        } else if (event.key === Qt.Key_Left) {
            if (root.selTab === 2)
                root.adjustVol(-10)
            else if (root.selTab === 3)
                root.recSelIndex = Math.max(0, root.recSelIndex - 1)
            else
                root.switchTab(-1)
            event.accepted = true
        } else if (event.key === Qt.Key_Right) {
            if (root.selTab === 2)
                root.adjustVol(10)
            else if (root.selTab === 3)
                root.recSelIndex = Math.min(AudioRecorderManager.recordings.length - 1, root.recSelIndex + 1)
            else
                root.switchTab(1)
            event.accepted = true
        } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            if (root.selTab === 2)
                root.toggleMute()
            else if (root.selTab === 3) {
                const recs = AudioRecorderManager.recordings
                if (recs.length > 0 && root.recSelIndex < recs.length) {
                    if (AudioRecorderManager.playProc.running)
                        AudioRecorderManager.stopPlayback()
                    else
                        AudioRecorderManager.play(recs[root.recSelIndex])
                }
            } else
                root.activate()
            event.accepted = true
        } else if (event.key === Qt.Key_M) {
            if (root.selTab === 2)
                root.toggleMute()
            event.accepted = true
        } else if (event.key === Qt.Key_Delete) {
            if (root.selTab === 3) {
                const recs = AudioRecorderManager.recordings
                if (recs.length > 0 && root.recSelIndex < recs.length)
                    AudioRecorderManager.del(recs[root.recSelIndex], true)
            }
            event.accepted = true
        }
    }

    // Floating OSD chip (accent pill) that hovers over the tab bar. visible
    // binds to the chip's own animated opacity so the fade-OUT plays before
    // hiding (binding to the osdOpacity property would hide it instantly).
    Rectangle {
        visible: opacity > 0
        opacity: root.osdOpacity
        width: osdLabel.implicitWidth + 30
        height: 30
        radius: 15
        color: Theme.acc
        z: 100
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top: parent.top
        anchors.topMargin: 48

        Behavior on opacity {
            NumberAnimation { duration: 150; easing.type: Easing.OutCubic }
        }

        Txt {
            id: osdLabel
            anchors.centerIn: parent
            text: root.osdText
            color: Theme.sfg
            font.family: Theme.fontName
            font.pixelSize: 12
            font.bold: true
        }
    }

    Column {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 12

        Row {
            width: parent.width
            spacing: 12

            Rectangle {
                width: 38
                height: 38
                radius: 19
                color: Theme.bg3

                StatusIcon {
                    anchors.centerIn: parent
                    type: "music"
                    iconSize: 20
                    color: Theme.fg
                }
            }

            Column {
                anchors.verticalCenter: parent.verticalCenter
                spacing: 1

                Txt {
                    text: "Audio"
                    color: Theme.fg
                    font.family: Theme.fontName
                    font.pixelSize: 15
                    font.bold: true
                }

                Txt {
                    text: root.selTab === 2
                        ? "Streams • ←/→ volume • Enter: mute • Tab: switch"
                        : root.selTab === 3
                            ? "Record • Enter: play/stop • Del: remove • Tab: switch"
                            : "Select output / input • Tab: switch • Enter: set default"
                    color: Theme.fg3
                    font.family: Theme.fontName
                    font.pixelSize: 11
                }
            }
        }

        // ---- Tab labels ----
        Row {
            width: parent.width
            spacing: 6

            Repeater {
                model: [
                    { key: 0, label: "Outputs" },
                    { key: 1, label: "Inputs" },
                    { key: 2, label: "Streams" },
                    { key: 3, label: "Record" }
                ]

                delegate: Rectangle {
                    required property var modelData
                    width: (parent.width - 18) / 4
                    height: 28
                    radius: 9
                    color: root.selTab === modelData.key ? Theme.acc : Theme.bg3
                    border.width: 1
                    border.color: root.selTab === modelData.key ? Theme.acc : Theme.border

                    Behavior on color {
                        ColorAnimation { duration: 120 }
                    }

                    Txt {
                        anchors.centerIn: parent
                        text: modelData.label
                        color: root.selTab === modelData.key ? Theme.sfg : Theme.fg2
                        font.family: Theme.fontName
                        font.pixelSize: 12
                        font.bold: root.selTab === modelData.key
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            root.selTab = modelData.key
                            root.selIndex = 0
                        }
                    }
                }
            }
        }

        Item {
            width: parent.width
            height: parent.height - 62 - 34
            clip: true

            // ---- Outputs / Inputs (two columns) ----
            Row {
                visible: root.selTab < 2
                width: parent.width
                height: parent.height
                spacing: 12

                // Outputs
                Item {
                    width: parent.width / 2 - 6
                    height: parent.height

                    Rectangle {
                        anchors.fill: parent
                        radius: 12
                        color: Theme.bg3
                        border.width: root.selTab === 0 ? 1 : 0
                        border.color: Theme.acc

                        Txt {
                            anchors.centerIn: parent
                            visible: AudioManager.sinks.length === 0
                            text: AudioManager.loading ? "Scanning..." : "No outputs"
                            color: Theme.fg3
                            font.family: Theme.fontName
                            font.pixelSize: 12
                        }

                        ListView {
                            id: sinkList
                            anchors.fill: parent
                            anchors.margins: 6
                            clip: true
                            model: AudioManager.sinks
                            currentIndex: root.selTab === 0 ? root.selIndex : -1

                            delegate: Item {
                                required property int index
                                required property var modelData
                                width: sinkList.width
                                height: 52

                                Rectangle {
                                    anchors.fill: parent
                                    radius: 10
                                    color: sinkList.currentIndex === index ? Theme.acc : "transparent"
                                    border.width: modelData.active ? 1 : 0
                                    border.color: Theme.sfg

                                    Behavior on color {
                                        ColorAnimation { duration: 120 }
                                    }

                                    Column {
                                        anchors.left: parent.left
                                        anchors.right: parent.right
                                        anchors.verticalCenter: parent.verticalCenter
                                        anchors.leftMargin: 10
                                        anchors.rightMargin: 10
                                        spacing: 2

                                        Txt {
                                            text: modelData.desc
                                            color: sinkList.currentIndex === index ? Theme.sfg : Theme.fg
                                            font.family: Theme.fontName
                                            font.pixelSize: 12
                                            font.bold: sinkList.currentIndex === index || modelData.active
                                            elide: Text.ElideRight
                                            width: parent.width
                                        }

                                        Row {
                                            visible: modelData.active || sinkList.currentIndex === index
                                            spacing: 6

                                            Row {
                                                spacing: 4

                                                StatusIcon {
                                                    visible: modelData.active
                                                    anchors.verticalCenter: parent.verticalCenter
                                                    type: "check"
                                                    iconSize: 12
                                                    color: sinkList.currentIndex === index ? Theme.sfg : Theme.acc
                                                }

                                                Txt {
                                                    visible: !modelData.active
                                                    text: "set"
                                                    font.family: Theme.fontName
                                                    font.pixelSize: 11
                                                    color: sinkList.currentIndex === index ? Theme.sfg : Theme.acc
                                                }
                                            }

                                            Txt {
                                                visible: modelData.active
                                                text: "default"
                                                font.family: Theme.fontName
                                                font.pixelSize: 10
                                                color: sinkList.currentIndex === index ? Theme.sfg : Theme.fg3
                                            }
                                        }
                                    }
                                }

                                MouseArea {
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onEntered: {
                                        if (root.selTab !== 0) root.selTab = 0
                                        root.selIndex = index
                                    }
                                    onClicked: {
                                        root.selTab = 0
                                        root.selIndex = index
                                        root.activate()
                                    }
                                }
                            }

                            onCurrentIndexChanged: {
                                positionViewAtIndex(currentIndex, ListView.Contain)
                            }
                        }
                    }
                }

                // Inputs
                Item {
                    width: parent.width / 2 - 6
                    height: parent.height

                    Rectangle {
                        anchors.fill: parent
                        radius: 12
                        color: Theme.bg3
                        border.width: root.selTab === 1 ? 1 : 0
                        border.color: Theme.acc

                        Txt {
                            anchors.centerIn: parent
                            visible: AudioManager.sources.length === 0
                            text: AudioManager.loading ? "Scanning..." : "No inputs"
                            color: Theme.fg3
                            font.family: Theme.fontName
                            font.pixelSize: 12
                        }

                        ListView {
                            id: sourceList
                            anchors.fill: parent
                            anchors.margins: 6
                            clip: true
                            model: AudioManager.sources
                            currentIndex: root.selTab === 1 ? root.selIndex : -1

                            delegate: Item {
                                required property int index
                                required property var modelData
                                width: sourceList.width
                                height: 52

                                Rectangle {
                                    anchors.fill: parent
                                    radius: 10
                                    color: sourceList.currentIndex === index ? Theme.acc : "transparent"
                                    border.width: modelData.active ? 1 : 0
                                    border.color: Theme.sfg

                                    Behavior on color {
                                        ColorAnimation { duration: 120 }
                                    }

                                    Column {
                                        anchors.left: parent.left
                                        anchors.right: parent.right
                                        anchors.verticalCenter: parent.verticalCenter
                                        anchors.leftMargin: 10
                                        anchors.rightMargin: 10
                                        spacing: 2

                                        Txt {
                                            text: modelData.desc
                                            color: sourceList.currentIndex === index ? Theme.sfg : Theme.fg
                                            font.family: Theme.fontName
                                            font.pixelSize: 12
                                            font.bold: sourceList.currentIndex === index || modelData.active
                                            elide: Text.ElideRight
                                            width: parent.width
                                        }

                                        Row {
                                            visible: modelData.active || sourceList.currentIndex === index
                                            spacing: 6

                                            Row {
                                                spacing: 4

                                                StatusIcon {
                                                    visible: modelData.active
                                                    anchors.verticalCenter: parent.verticalCenter
                                                    type: "check"
                                                    iconSize: 12
                                                    color: sourceList.currentIndex === index ? Theme.sfg : Theme.acc
                                                }

                                                Txt {
                                                    visible: !modelData.active
                                                    text: "set"
                                                    font.family: Theme.fontName
                                                    font.pixelSize: 11
                                                    color: sourceList.currentIndex === index ? Theme.sfg : Theme.acc
                                                }
                                            }

                                            Txt {
                                                visible: modelData.active
                                                text: "default"
                                                font.family: Theme.fontName
                                                font.pixelSize: 10
                                                color: sourceList.currentIndex === index ? Theme.sfg : Theme.fg3
                                            }
                                        }
                                    }
                                }

                                MouseArea {
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onEntered: {
                                        if (root.selTab !== 1) root.selTab = 1
                                        root.selIndex = index
                                    }
                                    onClicked: {
                                        root.selTab = 1
                                        root.selIndex = index
                                        root.activate()
                                    }
                                }
                            }

                            onCurrentIndexChanged: {
                                positionViewAtIndex(currentIndex, ListView.Contain)
                            }
                        }
                    }
                }
            }

            // ---- Per-app streams (full width) ----
            Item {
                visible: root.selTab === 2
                width: parent.width
                height: parent.height

                Rectangle {
                    anchors.fill: parent
                    radius: 12
                    color: Theme.bg3
                    border.width: 1
                    border.color: Theme.acc

                    Txt {
                        anchors.centerIn: parent
                        visible: AudioManager.streams.length === 0
                        text: AudioManager.loading ? "Scanning..." : "No apps playing audio"
                        color: Theme.fg3
                        font.family: Theme.fontName
                        font.pixelSize: 12
                    }

                    ListView {
                        id: streamList
                        anchors.fill: parent
                        anchors.margins: 6
                        clip: true
                        spacing: 2
                        model: AudioManager.streams
                        currentIndex: root.selTab === 2 ? root.selIndex : -1

                        delegate: Item {
                            required property int index
                            required property var modelData
                            width: streamList.width
                            height: 46

                            Rectangle {
                                anchors.fill: parent
                                radius: 10
                                color: streamList.currentIndex === index ? Theme.acc : "transparent"

                                Behavior on color {
                                    ColorAnimation { duration: 120 }
                                }

                                Row {
                                    anchors.fill: parent
                                    anchors.leftMargin: 12
                                    anchors.rightMargin: 12
                                    spacing: 10

                                    StatusIcon {
                                        anchors.verticalCenter: parent.verticalCenter
                                        type: "mic"
                                        muted: modelData.muted
                                        iconSize: 18
                                        color: streamList.currentIndex === index ? Theme.sfg
                                            : (modelData.muted ? Theme.danger : Theme.fg)
                                    }

                                    Column {
                                        anchors.verticalCenter: parent.verticalCenter
                                        width: 150
                                        spacing: 1

                                        Txt {
                                            width: parent.width
                                            text: modelData.app
                                            color: streamList.currentIndex === index ? Theme.sfg : Theme.fg
                                            font.family: Theme.fontName
                                            font.pixelSize: 12
                                            font.bold: streamList.currentIndex === index || modelData.muted
                                            elide: Text.ElideRight
                                        }

                                        Txt {
                                            text: modelData.muted ? "muted" : (modelData.vol + "%")
                                            color: streamList.currentIndex === index ? Theme.sfg : Theme.fg3
                                            font.family: Theme.fontName
                                            font.pixelSize: 10
                                        }
                                    }

                                    // Volume bar
                                    Rectangle {
                                        anchors.verticalCenter: parent.verticalCenter
                                        width: parent.width - 150 - 70
                                        height: 8
                                        radius: 4
                                        color: streamList.currentIndex === index ? Qt.rgba(0, 0, 0, 0.18) : Theme.bg2

                                        Rectangle {
                                            anchors.left: parent.left
                                            anchors.verticalCenter: parent.verticalCenter
                                            width: Math.max(4, parent.width * (modelData.muted ? 0 : Math.min(1, modelData.vol / 100)))
                                            height: parent.height
                                            radius: 4
                                            color: modelData.muted
                                                ? Theme.fg3
                                                : (streamList.currentIndex === index ? Theme.sfg : Theme.success)
                                        }
                                    }

                                    Txt {
                                        anchors.verticalCenter: parent.verticalCenter
                                        visible: streamList.currentIndex === index
                                        text: "←/→ vol · Enter mute"
                                        font.family: Theme.fontName
                                        font.pixelSize: 9
                                        color: Theme.sfg
                                    }
                                }
                            }

                            MouseArea {
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onEntered: {
                                    if (root.selTab !== 2) root.selTab = 2
                                    root.selIndex = index
                                }
                                onClicked: {
                                    root.selTab = 2
                                    root.selIndex = index
                                }
                                onWheel: wheel => {
                                    if (root.selTab !== 2) root.selTab = 2
                                    root.selIndex = index
                                    root.adjustVol(wheel.angleDelta.y > 0 ? 10 : -10)
                                }
                            }
                        }

                        onCurrentIndexChanged: {
                            positionViewAtIndex(currentIndex, ListView.Contain)
                        }
                    }
                }
            }

            // ---- Record tab ----
            Item {
                visible: root.selTab === 3
                width: parent.width
                height: parent.height

                Row {
                    visible: root.recorderPane !== "recordings"
                    anchors.fill: parent
                    spacing: 12

                    // Recorder card
                    Rectangle {
                        width: parent.width * 0.48
                        height: parent.height
                        radius: 12
                        color: Theme.bg3
                        border.width: 1
                        border.color: Theme.border

                        Column {
                            anchors.fill: parent
                            anchors.margins: 12
                            spacing: 10

                            Row {
                                width: parent.width
                                spacing: 8

                                Rectangle {
                                    width: 32
                                    height: 32
                                    radius: 16
                                    color: AudioRecorderManager.recording ? Theme.danger : Theme.acc

                                    StatusIcon {
                                        anchors.centerIn: parent
                                        type: "mic"
                                        iconSize: 16
                                        color: Theme.sfg
                                    }
                                }

                                Column {
                                    anchors.verticalCenter: parent.verticalCenter
                                    spacing: 1

                                    Txt {
                                        text: "Audio Recorder"
                                        color: Theme.fg
                                        font.family: Theme.fontName
                                        font.pixelSize: 13
                                        font.bold: true
                                    }

                                    Txt {
                                        text: AudioRecorderManager.recording
                                            ? AudioRecorderManager.formatElapsed(AudioRecorderManager.elapsed)
                                            : "Ready"
                                        color: AudioRecorderManager.recording ? Theme.danger : Theme.fg3
                                        font.family: Theme.fontName
                                        font.pixelSize: 10
                                    }
                                }
                            }

                            // Mic volume slider
                            Column {
                                visible: AudioRecorderManager.showMicSlider
                                width: parent.width
                                spacing: 4

                                Txt {
                                    text: "Mic Volume"
                                    color: Theme.fg3
                                    font.family: Theme.fontName
                                    font.pixelSize: 10
                                }

                                CcSlider {
                                    width: parent.width
                                    height: 36
                                    value: VolumeManager.micVolume
                                    iconType: "mic"
                                    fillColor: Theme.acc
                                    onSetValue: (v, live) => VolumeManager.setMicVolume(v, live)
                                }
                            }

                            // Record name input
                            Rectangle {
                                width: parent.width
                                height: 30
                                radius: 8
                                color: Theme.bg2
                                border.width: 1
                                border.color: nameInput.activeFocus ? Theme.acc : Theme.border

                                TxtInput {
                                    id: nameInput
                                    anchors.fill: parent
                                    anchors.leftMargin: 10
                                    anchors.rightMargin: 10
                                    color: Theme.fg
                                    font.family: Theme.fontName
                                    font.pixelSize: 11
                                    clip: true
                                    selectByMouse: true
                                    text: root.recName
                                    onTextChanged: root.recName = text

                                    Txt {
                                        visible: !nameInput.activeFocus && nameInput.text.length === 0
                                        anchors.fill: parent
                                        anchors.leftMargin: 10
                                        text: "Recording name (optional)"
                                        color: Theme.fg3
                                        font.family: Theme.fontName
                                        font.pixelSize: 11
                                        verticalAlignment: Text.AlignVCenter
                                    }
                                }
                            }

                            // Record button
                            Rectangle {
                                width: parent.width
                                height: 36
                                radius: 10
                                color: AudioRecorderManager.recording ? Theme.danger : Theme.acc

                                Behavior on color { ColorAnimation { duration: 150 } }

                                Txt {
                                    anchors.centerIn: parent
                                    text: AudioRecorderManager.recording ? "\uf04d   Stop" : "\uf130   Record"
                                    color: Theme.sfg
                                    font.family: Theme.fontName
                                    font.pixelSize: 12
                                    font.bold: true
                                }

                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        if (AudioRecorderManager.recording)
                                            AudioRecorderManager.stopRecording()
                                        else
                                            AudioRecorderManager.startRecording(root.recName)
                                    }
                                }
                            }
                        }
                    }

                    // Recordings list
                    Rectangle {
                        width: parent.width * 0.52 - 12
                        height: parent.height
                        radius: 12
                        color: Theme.bg3
                        border.width: 1
                        border.color: Theme.border

                        Column {
                            anchors.fill: parent
                            anchors.margins: 10
                            spacing: 6

                            Row {
                                width: parent.width

                                Txt {
                                    text: "Recordings"
                                    color: Theme.fg2
                                    font.family: Theme.fontName
                                    font.pixelSize: 11
                                    font.bold: true
                                }

                                Item { width: parent.width - recCountLabel.implicitWidth - chevronRec.implicitWidth - 30; height: 1 }

                                Txt {
                                    id: recCountLabel
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: AudioRecorderManager.recordings.length
                                    color: Theme.fg3
                                    font.family: Theme.fontName
                                    font.pixelSize: 10
                                }

                                Rectangle {
                                    id: chevronRec
                                    width: 22
                                    height: 22
                                    radius: 11
                                    color: Theme.bg2

                                    StatusIcon {
                                        anchors.centerIn: parent
                                        type: "chevronRight"
                                        iconSize: 12
                                        color: Theme.fg2
                                    }

                                    MouseArea {
                                        anchors.fill: parent
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: root.recorderPane = "recordings"
                                    }
                                }
                            }

                            ListView {
                                width: parent.width
                                height: parent.height - 28
                                clip: true
                                model: AudioRecorderManager.recordings
                                spacing: 2

                                delegate: Item {
                                    required property int index
                                    required property string modelData
                                    width: ListView.view.width
                                    height: 38

                                    Rectangle {
                                        anchors.fill: parent
                                        radius: 8
                                        color: root.recSelIndex === index ? Theme.acc : "transparent"

                                        Behavior on color { ColorAnimation { duration: 100 } }

                                        Row {
                                            anchors.fill: parent
                                            anchors.leftMargin: 8
                                            anchors.rightMargin: 8
                                            spacing: 6

                                            StatusIcon {
                                                anchors.verticalCenter: parent.verticalCenter
                                                type: "music"
                                                iconSize: 14
                                                color: root.recSelIndex === index ? Theme.sfg : Theme.fg3
                                            }

                                            Txt {
                                                anchors.verticalCenter: parent.verticalCenter
                                                width: parent.width - 60
                                                text: modelData.replace(/\.[^.]+$/, "")
                                                color: root.recSelIndex === index ? Theme.sfg : Theme.fg
                                                font.family: Theme.fontName
                                                font.pixelSize: 11
                                                elide: Text.ElideRight
                                            }
                                        }
                                    }

                                    MouseArea {
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onEntered: root.recSelIndex = index
                                        onClicked: root.recSelIndex = index
                                    }
                                }
                            }
                        }
                    }
                }

                // 2nd pane: recordings list
                Item {
                    visible: root.recorderPane === "recordings"
                    anchors.fill: parent

                    Column {
                        anchors.fill: parent
                        anchors.margins: 10
                        spacing: 8

                        Row {
                            width: parent.width
                            spacing: 8

                            Rectangle {
                                width: 24
                                height: 24
                                radius: 12
                                color: Theme.bg3

                                StatusIcon {
                                    anchors.centerIn: parent
                                    type: "chevronLeft"
                                    iconSize: 12
                                    color: Theme.fg2
                                }

                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: root.recorderPane = ""
                                }
                            }

                            Txt {
                                anchors.verticalCenter: parent.verticalCenter
                                text: "All Recordings"
                                color: Theme.fg
                                font.family: Theme.fontName
                                font.pixelSize: 13
                                font.bold: true
                            }

                            Item { width: parent.width - 120; height: 1 }

                            // Confirm delete toggle
                            Row {
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 4

                                Txt {
                                    text: "Confirm"
                                    color: Theme.fg3
                                    font.family: Theme.fontName
                                    font.pixelSize: 9
                                }

                                ToggleSwitch {
                                    checked: AudioRecorderManager.confirmDelete
                                    onToggled: value => AudioRecorderManager.setCfg("confirmDelete", value)
                                }
                            }

                            // Mic slider toggle
                            Row {
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 4

                                Txt {
                                    text: "Mic"
                                    color: Theme.fg3
                                    font.family: Theme.fontName
                                    font.pixelSize: 9
                                }

                                ToggleSwitch {
                                    checked: AudioRecorderManager.showMicSlider
                                    onToggled: value => AudioRecorderManager.setCfg("showMicSlider", value)
                                }
                            }
                        }

                        Rectangle {
                            width: parent.width
                            height: parent.height - 40
                            radius: 12
                            color: Theme.bg3
                            border.width: 1
                            border.color: Theme.border

                            ListView {
                                anchors.fill: parent
                                anchors.margins: 6
                                clip: true
                                model: AudioRecorderManager.recordings
                                spacing: 2

                                delegate: Item {
                                    required property int index
                                    required property string modelData
                                    width: ListView.view.width
                                    height: 50

                                    Rectangle {
                                        anchors.fill: parent
                                        radius: 10
                                        color: root.recSelIndex === index ? Theme.acc : "transparent"

                                        Behavior on color { ColorAnimation { duration: 100 } }

                                        Row {
                                            anchors.fill: parent
                                            anchors.leftMargin: 10
                                            anchors.rightMargin: 10
                                            spacing: 8

                                            StatusIcon {
                                                anchors.verticalCenter: parent.verticalCenter
                                                type: AudioRecorderManager.playProc.running ? "pause" : "music"
                                                iconSize: 16
                                                color: root.recSelIndex === index ? Theme.sfg : Theme.fg3
                                            }

                                            Column {
                                                anchors.verticalCenter: parent.verticalCenter
                                                width: parent.width - 120
                                                spacing: 1

                                                Txt {
                                                    width: parent.width
                                                    text: modelData.replace(/\.[^.]+$/, "")
                                                    color: root.recSelIndex === index ? Theme.sfg : Theme.fg
                                                    font.family: Theme.fontName
                                                    font.pixelSize: 12
                                                    font.bold: root.recSelIndex === index
                                                    elide: Text.ElideRight
                                                }

                                                Txt {
                                                    text: modelData.split(".").pop().toUpperCase()
                                                    color: root.recSelIndex === index ? Theme.sfg : Theme.fg3
                                                    font.family: Theme.fontName
                                                    font.pixelSize: 9
                                                }
                                            }

                                            Item { width: parent.width - 170; height: 1 }

                                            // Play button
                                            Rectangle {
                                                width: 28
                                                height: 28
                                                radius: 14
                                                color: AudioRecorderManager.playProc.running ? Theme.acc : Theme.bg2

                                                StatusIcon {
                                                    anchors.centerIn: parent
                                                    type: AudioRecorderManager.playProc.running ? "pause" : "play"
                                                    iconSize: 12
                                                    color: AudioRecorderManager.playProc.running ? Theme.sfg : Theme.fg2
                                                }

                                                MouseArea {
                                                    anchors.fill: parent
                                                    cursorShape: Qt.PointingHandCursor
                                                    onClicked: {
                                                        if (AudioRecorderManager.playProc.running)
                                                            AudioRecorderManager.stopPlayback()
                                                        else
                                                            AudioRecorderManager.play(modelData)
                                                    }
                                                }
                                            }

                                            // Delete button
                                            Rectangle {
                                                width: 28
                                                height: 28
                                                radius: 14
                                                color: Theme.bg2

                                                StatusIcon {
                                                    anchors.centerIn: parent
                                                    type: "trash"
                                                    iconSize: 12
                                                    color: Theme.danger
                                                }

                                                MouseArea {
                                                    anchors.fill: parent
                                                    cursorShape: Qt.PointingHandCursor
                                                    onClicked: AudioRecorderManager.del(modelData, false)
                                                }
                                            }
                                        }
                                    }

                                    MouseArea {
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onEntered: root.recSelIndex = index
                                        onClicked: root.recSelIndex = index
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
