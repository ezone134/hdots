import QtQuick
import QtQuick.Layouts
import Quickshell.Io
import "."
import "../../components"
import "../../services"

// Control Center — shippable Apple-inspired quick settings.
// Hierarchy: connectivity modules → utility tiles → thick sliders → media → lists.
Item {
    id: root
    implicitWidth: 380
    implicitHeight: mainCol.implicitHeight + 40
    focus: true

    property bool showPassword: false
    property string passwordTarget: ""
    property string passwordText: ""
    property bool nightLightOn: false
    property int colorTemp: 4000
    property bool _tempDrag: false
    property bool caffeineOn: false
    property bool ppAvailable: false
    property string powerProfile: ""
    property real shaderVibrance: 1.0
    property bool shaderOn: false
    property bool _shaderDrag: false
    property string osdIcon: ""
    property string osdText: ""
    property real osdOpacity: 0
    property string errorText: ""
    // "" = home, "wifi" / "bt" = Apple-style expanded sheets
    property string detailPage: ""

    function openWifiDetail() {
        root.detailPage = "wifi"
        NetworkManager.refreshWifi()
    }

    function openBtDetail() {
        root.detailPage = "bt"
        NetworkManager.refreshBluetooth()
    }

    function closeDetail() {
        root.detailPage = ""
    }

    property Timer errorTimer: Timer {
        interval: 3200
        onTriggered: errorText = ""
    }

    Timer {
        id: osdTimer
        interval: 900
        repeat: false
        onTriggered: root.osdOpacity = 0
    }

    function showOsd(iconType, text) {
        root.osdIcon = iconType
        root.osdText = text
        root.osdOpacity = 1
        osdTimer.restart()
    }

    function pctLabel(p) {
        return Math.round(Math.max(0, Math.min(1, p)) * 100) + "%"
    }

    function profileLabel() {
        if (root.powerProfile === "performance")
            return "Performance"
        if (root.powerProfile === "power-saver")
            return "Low Power"
        if (root.powerProfile === "balanced")
            return "Balanced"
        return root.powerProfile.length ? root.powerProfile : "Power"
    }

    function wifiSubtitle() {
        if (!NetworkManager.wifiEnabled)
            return "Off"
        if (NetworkManager.currentWifi && NetworkManager.currentWifi.name)
            return NetworkManager.currentWifi.name
        return "Not Connected"
    }

    function btSubtitle() {
        if (!NetworkManager.bluetoothEnabled)
            return "Off"
        const devs = NetworkManager.btDevices || []
        for (let i = 0; i < devs.length; i++) {
            if (devs[i].connected)
                return devs[i].name || "Connected"
        }
        return "On"
    }

    function shaderPathForVibrance(v) {
        const n = Math.max(0, Math.min(20, Math.round(v * 10)))
        return SystemSettingsManager.shaders + "/saturations/" + n + ".glsl"
    }

    function vibranceFromShaderPath(p) {
        const s = String(p || "").trim()
        const base = s.split("/").pop() || ""
        const num = parseInt(base.replace(".glsl", ""), 10)
        if (isNaN(num))
            return 1.0
        return Math.max(0, Math.min(2, num / 10))
    }

    function shaderLabel(v) {
        const n = Math.max(0, Math.min(20, Math.round(v * 10)))
        if (n === 0)
            return "Gray"
        if (n === 10)
            return "Standard"
        return (n / 10).toFixed(1) + "×"
    }

    function refreshShader() {
        const ch = SystemSettingsManager.channel
        const st = SystemSettingsManager.states
        SystemSettingsManager.readBatch([
            st + "/shader_state_" + ch,
            st + "/shader_val_" + ch
        ], r => {
            if (root._shaderDrag)
                return
            root.shaderOn = (r[0] || "0").trim() === "1"
            const path = (r[1] || "").trim()
            if (path.length > 0)
                root.shaderVibrance = root.vibranceFromShaderPath(path)
            else if (!root.shaderOn)
                root.shaderVibrance = 1.0
        })
    }

    function applyShaderVibrance(v) {
        root.shaderVibrance = Math.max(0, Math.min(2, v))
        const n = Math.max(0, Math.min(20, Math.round(root.shaderVibrance * 10)))
        const bin = SystemSettingsManager.qsScripts + "/shader_main"
        SystemSettingsManager.apply(bin, "set " + n)
        root.shaderOn = true
    }

    function setVibranceFromUnit(u, commit) {
        const v = Math.max(0, Math.min(1, u)) * 2.0
        root.shaderVibrance = v
        root.showOsd("vibrance", "Vibrance  " + root.shaderLabel(v))
        if (commit)
            root.applyShaderVibrance(v)
        else
            root.shaderOn = true
    }

    function refreshQuick() {
        SystemSettingsManager.readBatch([SystemSettingsManager.states + "/hyprsunset_state"], r => {
            const v = (r[0] || "").trim()
            root.nightLightOn = v.length > 0 && v !== "c"
        })
        caffeineStatusProc.running = false
        caffeineStatusProc.running = true
        ppGetProc.running = false
        ppGetProc.running = true
    }

    function toggleNightLight() {
        const bin = SystemSettingsManager.qsScripts + "/hyprsunset_main"
        SystemSettingsManager.apply(bin, "toggle")
    }

    function refreshColorTemp() {
        const st = SystemSettingsManager.states
        SystemSettingsManager.readBatch([
            st + "/hyprsunset_state",
            st + "/hyprsunset_val"
        ], r => {
            if (root._tempDrag)
                return
            const s = (r[0] || "").trim()
            root.nightLightOn = s.length > 0 && s !== "c"
            const v = parseInt((r[1] || "4000").trim(), 10)
            root.colorTemp = isNaN(v) ? 4000 : Math.max(1200, Math.min(6500, v))
        })
    }

    function applyColorTemp(k) {
        let n = Math.round(k / 100) * 100
        n = Math.max(1200, Math.min(6500, n))
        root.colorTemp = n
        const st = SystemSettingsManager.states
        const bin = SystemSettingsManager.qsScripts + "/hyprsunset_main"
        SystemSettingsManager.writeFiles([
            { path: st + "/hyprsunset_val", value: String(n) }
        ], () => {
            SystemSettingsManager.apply(bin, "manual")
            root.nightLightOn = true
        })
    }

    function setTempFromUnit(u, commit) {
        const clamped = Math.max(0, Math.min(1, u))
        const k = 1200 + clamped * (6500 - 1200)
        const snapped = Math.round(k / 100) * 100
        root.colorTemp = Math.max(1200, Math.min(6500, snapped))
        root.showOsd("warmth", "Warmth  " + root.colorTemp + "K")
        if (commit)
            root.applyColorTemp(root.colorTemp)
        else
            root.nightLightOn = true
    }

    function toggleCaffeine() {
        caffeineToggleProc.running = false
        caffeineToggleProc.running = true
    }

    function cyclePowerProfile() {
        ppCycleProc.running = false
        ppCycleProc.running = true
    }

    function togglePassword(ssid) {
        root.passwordTarget = ssid
        root.passwordText = ""
        root.showPassword = true
        passInput.forceActiveFocus()
    }

    function submitPassword() {
        if (root.passwordText.length === 0)
            return
        NetworkManager.connectWifiPassword(root.passwordTarget, root.passwordText)
        root.showPassword = false
    }

    Process {
        id: caffeineStatusProc
        command: ["caffeine_main", "status"]
        stdout: SplitParser {
            onRead: line => {
                root.caffeineOn = String(line).trim() === "on"
            }
        }
    }

    Process {
        id: caffeineToggleProc
        command: ["caffeine_main", "toggle"]
        onExited: {
            caffeineStatusProc.running = false
            caffeineStatusProc.running = true
        }
    }

    Process {
        id: ppGetProc
        command: ["bash", "-c", "command -v powerprofilesctl >/dev/null 2>&1 && powerprofilesctl get 2>/dev/null || true"]
        stdout: StdioCollector {
            onStreamFinished: {
                const out = String(text).trim()
                if (out.length > 0) {
                    root.ppAvailable = true
                    root.powerProfile = out
                } else {
                    root.ppAvailable = false
                    root.powerProfile = ""
                }
            }
        }
    }

    Process {
        id: ppCycleProc
        command: ["pp_main", "cycle"]
        onExited: {
            ppGetProc.running = false
            ppGetProc.running = true
        }
    }

    Connections {
        target: SystemSettingsManager
        function onApplied() {
            SystemSettingsManager.readBatch([SystemSettingsManager.states + "/hyprsunset_state"], r => {
                const v = (r[0] || "").trim()
                root.nightLightOn = v.length > 0 && v !== "c"
            })
            root._shaderDrag = false
            root._tempDrag = false
            root.refreshShader()
            root.refreshColorTemp()
        }
        function onChannelChanged() {
            root.refreshShader()
        }
    }

    OverlayFocusScope {
        delay: 140
        focusTarget: root
    }

    onVisibleChanged: {
        if (visible) {
            NetworkManager.refreshWifi()
            NetworkManager.refreshBluetooth()
            root.refreshQuick()
            root.refreshShader()
            root.refreshColorTemp()
        } else {
            root.detailPage = ""
        }
    }

    Connections {
        target: NetworkManager
        function onPromptPassword(ssid) {
            root.togglePassword(ssid)
        }
        function onSignalError(msg) {
            errorText = msg
            errorTimer.restart()
        }
    }

    // Surface
    Rectangle {
        anchors.fill: parent
        radius: 28
        color: Theme.bg
        border.width: 1
        border.color: Theme.border
    }

    // Password sheet
    Rectangle {
        id: passwordOverlay
        anchors.fill: parent
        radius: 28
        visible: root.showPassword
        color: Qt.rgba(0, 0, 0, 0.45)
        z: 50

        MouseArea {
            anchors.fill: parent
            onClicked: root.showPassword = false
        }

        Rectangle {
            width: Math.min(340, parent.width - 32)
            height: 168
            anchors.centerIn: parent
            radius: 20
            color: Theme.bg2
            border.color: Theme.border
            border.width: 1

            Column {
                anchors.fill: parent
                anchors.margins: 20
                spacing: 14

                Txt {
                    text: "Join “" + root.passwordTarget + "”"
                    color: Theme.fg
                    font.family: Theme.fontName
                    font.pixelSize: 15
                    font.bold: true
                    elide: Text.ElideRight
                    width: parent.width
                }

                Rectangle {
                    width: parent.width
                    height: 40
                    radius: 12
                    color: Theme.bg3
                    border.width: 1
                    border.color: Theme.border

                    TxtInput {
                        id: passInput
                        anchors.fill: parent
                        anchors.leftMargin: 12
                        anchors.rightMargin: 12
                        verticalAlignment: TextInput.AlignVCenter
                        color: Theme.fg
                        font.family: Theme.fontName
                        font.pixelSize: 13
                        echoMode: TextInput.Password
                        selectionColor: Theme.acc
                        text: root.passwordText
                        onTextChanged: root.passwordText = text
                        Keys.onEscapePressed: function (event) {
                            root.showPassword = false
                            event.accepted = true
                        }
                        Keys.onReturnPressed: function (event) {
                            root.submitPassword()
                            event.accepted = true
                        }
                    }
                }

                Row {
                    width: parent.width
                    spacing: 10

                    Rectangle {
                        width: parent.width / 2 - 5
                        height: 36
                        radius: 12
                        color: Theme.bg3

                        Txt {
                            anchors.centerIn: parent
                            text: "Cancel"
                            color: Theme.fg
                            font.family: Theme.fontName
                            font.pixelSize: 13
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.showPassword = false
                        }
                    }

                    Rectangle {
                        width: parent.width / 2 - 5
                        height: 36
                        radius: 12
                        color: Theme.acc

                        Txt {
                            anchors.centerIn: parent
                            text: "Join"
                            color: Theme.sfg
                            font.family: Theme.fontName
                            font.pixelSize: 13
                            font.bold: true
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.submitPassword()
                        }
                    }
                }
            }
        }
    }

    // Floating value chip
    Rectangle {
        visible: opacity > 0
        opacity: root.osdOpacity
        width: osdChipRow.implicitWidth + 24
        height: 28
        radius: 14
        color: Theme.fg
        z: 100
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top: parent.top
        anchors.topMargin: 14

        Behavior on opacity {
            NumberAnimation {
                duration: 150
                easing.type: Easing.OutCubic
            }
        }

        Row {
            id: osdChipRow
            anchors.centerIn: parent
            spacing: 6

            StatusIcon {
                anchors.verticalCenter: parent.verticalCenter
                type: root.osdIcon
                iconSize: 14
                color: Theme.bg
            }

            Txt {
                text: root.osdText
                color: Theme.bg
                font.family: Theme.fontName
                font.pixelSize: 12
                font.bold: true
            }
        }
    }

    Column {
        id: mainCol
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 16
        spacing: 12

        // —— Home ——
        Column {
            width: parent.width
            spacing: 12
            visible: root.detailPage === ""

            RowLayout {
                width: parent.width
                spacing: 10

                Txt {
                    text: "Control Center"
                    color: Theme.fg
                    font.family: Theme.fontName
                    font.pixelSize: 17
                    font.bold: true
                    Layout.fillWidth: true
                }

                Txt {
                    visible: root.errorText.length > 0
                    text: root.errorText
                    color: Theme.danger
                    font.family: Theme.fontName
                    font.pixelSize: 11
                    elide: Text.ElideRight
                    Layout.maximumWidth: 140
                }
            }

            // Connectivity: body = toggle, › = detail sheet
            Row {
                width: parent.width
                spacing: 10

                CcTile {
                    width: (parent.width - 10) / 2
                    height: 76
                    iconType: "wifi"
                    title: "Wi‑Fi"
                    subtitle: root.wifiSubtitle()
                    active: NetworkManager.wifiEnabled
                    expandable: true
                    onClicked: NetworkManager.toggleWifi()
                    onExpandClicked: root.openWifiDetail()
                }

                CcTile {
                    width: (parent.width - 10) / 2
                    height: 76
                    iconType: "bluetooth"
                    title: "Bluetooth"
                    subtitle: root.btSubtitle()
                    active: NetworkManager.bluetoothEnabled
                    expandable: true
                    onClicked: NetworkManager.toggleBluetooth()
                    onExpandClicked: root.openBtDetail()
                }
            }
        }

        // —— Wi‑Fi detail sheet ——
        Column {
            width: parent.width
            spacing: 12
            visible: root.detailPage === "wifi"

            RowLayout {
                width: parent.width
                spacing: 8

                Rectangle {
                    Layout.preferredWidth: 32
                    Layout.preferredHeight: 32
                    radius: 16
                    color: Theme.bg3

                    StatusIcon {
                        anchors.centerIn: parent
                        type: "chevronLeft"
                        iconSize: 16
                        color: Theme.fg
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.closeDetail()
                    }
                }

                Column {
                    Layout.fillWidth: true
                    spacing: 1

                    Txt {
                        text: "Wi‑Fi"
                        color: Theme.fg
                        font.family: Theme.fontName
                        font.pixelSize: 16
                        font.bold: true
                    }

                    Txt {
                        text: root.wifiSubtitle()
                        color: Theme.fg3
                        font.family: Theme.fontName
                        font.pixelSize: 11
                    }
                }

                Rectangle {
                    Layout.preferredWidth: 52
                    Layout.preferredHeight: 28
                    radius: 14
                    color: NetworkManager.wifiEnabled ? Theme.acc : Theme.bg3

                    Txt {
                        anchors.centerIn: parent
                        text: NetworkManager.wifiEnabled ? "On" : "Off"
                        color: NetworkManager.wifiEnabled ? Theme.sfg : Theme.fg2
                        font.family: Theme.fontName
                        font.pixelSize: 11
                        font.bold: true
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: NetworkManager.toggleWifi()
                    }
                }
            }

            CcSection {
                width: parent.width
                title: "NETWORKS"
                iconType: "wifi"
                trailing: NetworkManager.currentWifi && NetworkManager.currentWifi.name ? NetworkManager.currentWifi.name : ""
                scanning: NetworkManager.scanning
                listHeight: 320
                onRefreshClicked: NetworkManager.refreshWifi()

                ListView {
                    anchors.fill: parent
                    clip: true
                    spacing: 2
                    model: NetworkManager.wifiNetworks

                    Txt {
                        anchors.centerIn: parent
                        visible: NetworkManager.wifiNetworks.length === 0
                        text: !NetworkManager.wifiEnabled ? "Wi‑Fi Off" : (NetworkManager.scanning ? "Searching…" : "No Networks")
                        color: Theme.fg3
                        font.family: Theme.fontName
                        font.pixelSize: 12
                    }

                    delegate: Item {
                        required property var modelData
                        width: ListView.view.width
                        height: 44

                        Rectangle {
                            anchors.fill: parent
                            radius: 12
                            color: modelData.active ? Qt.rgba(Theme.acc.r, Theme.acc.g, Theme.acc.b, 0.18) : "transparent"

                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 10
                                anchors.rightMargin: 8
                                spacing: 8

                                StatusIcon {
                                    Layout.alignment: Qt.AlignVCenter
                                    type: "wifi"
                                    level: modelData.signal / 100
                                    active: true
                                    iconSize: 18
                                    color: modelData.active ? Theme.acc : Theme.fg2
                                }

                                Column {
                                    Layout.fillWidth: true
                                    spacing: 1

                                    Txt {
                                        width: parent.width
                                        text: modelData.name
                                        color: modelData.active ? Theme.acc : Theme.fg
                                        font.family: Theme.fontName
                                        font.pixelSize: 12
                                        font.bold: modelData.active
                                        elide: Text.ElideRight
                                    }

                                    Txt {
                                        visible: modelData.active || modelData.connecting || modelData.secured
                                        text: modelData.connecting ? "Joining…" : (modelData.active ? "Connected" : (modelData.secured ? "Secured" : "Open"))
                                        color: Theme.fg3
                                        font.family: Theme.fontName
                                        font.pixelSize: 10
                                    }
                                }

                                Rectangle {
                                    Layout.preferredWidth: modelData.active ? 72 : 64
                                    Layout.preferredHeight: 26
                                    Layout.alignment: Qt.AlignVCenter
                                    radius: 13
                                    color: modelData.active ? "transparent" : Theme.acc
                                    border.width: modelData.active ? 1 : 0
                                    border.color: Theme.acc

                                    Txt {
                                        anchors.centerIn: parent
                                        text: modelData.active ? "Disconnect" : (modelData.connecting ? "…" : "Join")
                                        color: modelData.active ? Theme.acc : Theme.sfg
                                        font.family: Theme.fontName
                                        font.pixelSize: 10
                                        font.bold: true
                                    }

                                    MouseArea {
                                        anchors.fill: parent
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: {
                                            if (modelData.active)
                                                NetworkManager.disconnectWifi()
                                            else
                                                NetworkManager.connectWifi(modelData.name)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // —— Bluetooth detail sheet ——
        Column {
            width: parent.width
            spacing: 12
            visible: root.detailPage === "bt"

            RowLayout {
                width: parent.width
                spacing: 8

                Rectangle {
                    Layout.preferredWidth: 32
                    Layout.preferredHeight: 32
                    radius: 16
                    color: Theme.bg3

                    StatusIcon {
                        anchors.centerIn: parent
                        type: "chevronLeft"
                        iconSize: 16
                        color: Theme.fg
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.closeDetail()
                    }
                }

                Column {
                    Layout.fillWidth: true
                    spacing: 1

                    Txt {
                        text: "Bluetooth"
                        color: Theme.fg
                        font.family: Theme.fontName
                        font.pixelSize: 16
                        font.bold: true
                    }

                    Txt {
                        text: root.btSubtitle()
                        color: Theme.fg3
                        font.family: Theme.fontName
                        font.pixelSize: 11
                    }
                }

                Rectangle {
                    Layout.preferredWidth: 52
                    Layout.preferredHeight: 28
                    radius: 14
                    color: NetworkManager.bluetoothEnabled ? Theme.acc : Theme.bg3

                    Txt {
                        anchors.centerIn: parent
                        text: NetworkManager.bluetoothEnabled ? "On" : "Off"
                        color: NetworkManager.bluetoothEnabled ? Theme.sfg : Theme.fg2
                        font.family: Theme.fontName
                        font.pixelSize: 11
                        font.bold: true
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: NetworkManager.toggleBluetooth()
                    }
                }
            }

            CcSection {
                width: parent.width
                title: "DEVICES"
                iconType: "bluetooth"
                scanning: NetworkManager.btScanning
                listHeight: 320
                onRefreshClicked: NetworkManager.refreshBluetooth()

                ListView {
                    anchors.fill: parent
                    clip: true
                    spacing: 2
                    model: NetworkManager.btDevices

                    Txt {
                        anchors.centerIn: parent
                        visible: NetworkManager.btDevices.length === 0
                        text: !NetworkManager.bluetoothEnabled ? "Bluetooth Off" : "No Devices"
                        color: Theme.fg3
                        font.family: Theme.fontName
                        font.pixelSize: 12
                    }

                    delegate: Item {
                        required property var modelData
                        width: ListView.view.width
                        height: 44

                        Rectangle {
                            anchors.fill: parent
                            radius: 12
                            color: modelData.connected ? Qt.rgba(Theme.acc.r, Theme.acc.g, Theme.acc.b, 0.18) : "transparent"

                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 10
                                anchors.rightMargin: 8
                                spacing: 8

                                StatusIcon {
                                    Layout.alignment: Qt.AlignVCenter
                                    type: "bluetooth"
                                    active: modelData.connected
                                    iconSize: 18
                                    color: modelData.connected ? Theme.acc : Theme.fg2
                                }

                                Column {
                                    Layout.fillWidth: true
                                    spacing: 1

                                    Txt {
                                        width: parent.width
                                        text: modelData.name
                                        color: modelData.connected ? Theme.acc : Theme.fg
                                        font.family: Theme.fontName
                                        font.pixelSize: 12
                                        font.bold: modelData.connected
                                        elide: Text.ElideRight
                                    }

                                    Txt {
                                        visible: modelData.connected || modelData.connecting
                                        text: modelData.connecting ? "Connecting…" : "Connected"
                                        color: Theme.fg3
                                        font.family: Theme.fontName
                                        font.pixelSize: 10
                                    }
                                }

                                Rectangle {
                                    Layout.preferredWidth: modelData.connected ? 72 : 64
                                    Layout.preferredHeight: 26
                                    Layout.alignment: Qt.AlignVCenter
                                    radius: 13
                                    color: modelData.connected ? "transparent" : Theme.acc
                                    border.width: modelData.connected ? 1 : 0
                                    border.color: Theme.acc

                                    Txt {
                                        anchors.centerIn: parent
                                        text: modelData.connected ? "Disconnect" : (modelData.connecting ? "…" : "Connect")
                                        color: modelData.connected ? Theme.acc : Theme.sfg
                                        font.family: Theme.fontName
                                        font.pixelSize: 10
                                        font.bold: true
                                    }

                                    MouseArea {
                                        anchors.fill: parent
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: {
                                            if (modelData.connected)
                                                NetworkManager.disconnectBluetooth(modelData.address)
                                            else
                                                NetworkManager.connectBluetooth(modelData.address)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Rest of home (hidden on detail pages)
        Column {
            width: parent.width
            spacing: 12
            visible: root.detailPage === ""

        // Utility modules — 3-up like iOS CC density
        GridLayout {
            width: parent.width
            columns: 3
            rowSpacing: 10
            columnSpacing: 10

            CcTile {
                Layout.fillWidth: true
                Layout.preferredHeight: 70
                iconType: "bell"
                title: NotificationManager.dndEnabled ? "Focused" : "Focus"
                active: NotificationManager.dndEnabled
                onClicked: NotificationManager.toggleDnd()
            }

            CcTile {
                Layout.fillWidth: true
                Layout.preferredHeight: 70
                iconType: "mic"
                title: VolumeManager.micMuted ? "Mic Off" : "Mic"
                active: !VolumeManager.micMuted
                muted: VolumeManager.micMuted
                onClicked: VolumeManager.toggleMic()
            }

            CcTile {
                Layout.fillWidth: true
                Layout.preferredHeight: 70
                iconType: Theme.mode === "dark" ? "moon" : "sun"
                title: Theme.mode === "dark" ? "Dark" : "Light"
                active: Theme.mode === "dark"
                onClicked: Theme.toggleMode()
            }

            CcTile {
                Layout.fillWidth: true
                Layout.preferredHeight: 70
                iconType: "bulb"
                title: "Night"
                active: root.nightLightOn
                onClicked: root.toggleNightLight()
            }

            CcTile {
                Layout.fillWidth: true
                Layout.preferredHeight: 70
                iconType: "coffee"
                title: root.caffeineOn ? "Awake" : "Caffeine"
                active: root.caffeineOn
                onClicked: root.toggleCaffeine()
            }

            CcTile {
                Layout.fillWidth: true
                Layout.preferredHeight: 70
                visible: root.ppAvailable
                iconType: "bolt"
                title: root.profileLabel()
                active: root.powerProfile.length > 0 && root.powerProfile !== "balanced"
                onClicked: root.cyclePowerProfile()
            }
        }

        // Five thin vertical sliders in one row
        Row {
            width: parent.width
            spacing: 8

            CcSlider {
                width: (parent.width - 32) / 5
                height: 156
                vertical: true
                barWidth: width
                iconSize: 14
                value: BrightnessManager.brightnessValue
                iconType: "sun"
                fillColor: Theme.fg
                onSetValue: (v, live) => {
                    BrightnessManager.setBrightness(v, live)
                    root.showOsd("sun", "Brightness  " + root.pctLabel(v))
                }
            }

            CcSlider {
                width: (parent.width - 32) / 5
                height: 156
                vertical: true
                barWidth: width
                iconSize: 14
                value: VolumeManager.volumeValue
                iconType: "volume"
                muted: VolumeManager.volumeValue <= 0
                fillColor: Theme.fg
                onSetValue: (v, live) => {
                    VolumeManager.setVolume(v, live)
                    root.showOsd("volume", "Volume  " + root.pctLabel(v))
                }
            }

            CcSlider {
                width: (parent.width - 32) / 5
                height: 156
                vertical: true
                barWidth: width
                iconSize: 14
                value: VolumeManager.micVolume
                iconType: "mic"
                muted: VolumeManager.micMuted
                fillColor: Theme.fg
                onSetValue: (v, live) => {
                    VolumeManager.setMicVolume(v, live)
                    root.showOsd("mic", "Microphone  " + root.pctLabel(v))
                }
            }

            CcSlider {
                width: (parent.width - 32) / 5
                height: 156
                vertical: true
                barWidth: width
                iconSize: 14
                value: (root.colorTemp - 1200) / (6500 - 1200)
                iconType: "warmth"
                muted: !root.nightLightOn
                fillColor: root.nightLightOn ? Theme.warning : Theme.fg3
                onSetValue: (v, live) => {
                    root._tempDrag = true
                    root.setTempFromUnit(v, !live)
                    if (!live)
                        root._tempDrag = false
                }
            }

            CcSlider {
                width: (parent.width - 32) / 5
                height: 156
                vertical: true
                barWidth: width
                iconSize: 14
                value: root.shaderVibrance / 2.0
                iconType: "vibrance"
                muted: !root.shaderOn
                fillColor: root.shaderOn ? Theme.fg : Theme.fg3
                onSetValue: (v, live) => {
                    root._shaderDrag = true
                    root.setVibranceFromUnit(v, !live)
                    if (!live)
                        root._shaderDrag = false
                }
            }
        }

        // Now Playing
        Rectangle {
            width: parent.width
            height: 64
            radius: 18
            color: Theme.bg3
            border.width: 1
            border.color: Theme.border

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 12
                spacing: 10

                Column {
                    Layout.fillWidth: true
                    spacing: 2

                    Txt {
                        width: parent.width
                        text: MprisManager.available ? MprisManager.title : "Not Playing"
                        color: MprisManager.available ? Theme.fg : Theme.fg3
                        font.family: Theme.fontName
                        font.pixelSize: 13
                        font.bold: MprisManager.available
                        elide: Text.ElideRight
                    }

                    Txt {
                        width: parent.width
                        visible: MprisManager.available
                        text: MprisManager.artist
                        color: Theme.fg3
                        font.family: Theme.fontName
                        font.pixelSize: 11
                        elide: Text.ElideRight
                    }
                }

                Row {
                    spacing: 6
                    Layout.alignment: Qt.AlignVCenter

                    Rectangle {
                        width: 32
                        height: 32
                        radius: 16
                        color: Theme.bg2
                        opacity: MprisManager.available ? 1 : 0.4

                        StatusIcon {
                            anchors.centerIn: parent
                            type: "prev"
                            iconSize: 14
                            color: Theme.fg
                        }

                        MouseArea {
                            anchors.fill: parent
                            enabled: MprisManager.available
                            cursorShape: Qt.PointingHandCursor
                            onClicked: MprisManager.previous()
                        }
                    }

                    Rectangle {
                        width: 36
                        height: 36
                        radius: 18
                        color: MprisManager.status === "Playing" ? Theme.fg : Theme.bg2
                        opacity: MprisManager.available ? 1 : 0.4

                        StatusIcon {
                            anchors.centerIn: parent
                            type: MprisManager.status === "Playing" ? "pause" : "play"
                            iconSize: 16
                            color: MprisManager.status === "Playing" ? Theme.bg : Theme.fg
                        }

                        MouseArea {
                            anchors.fill: parent
                            enabled: MprisManager.available
                            cursorShape: Qt.PointingHandCursor
                            onClicked: MprisManager.playPause()
                        }
                    }

                    Rectangle {
                        width: 32
                        height: 32
                        radius: 16
                        color: Theme.bg2
                        opacity: MprisManager.available ? 1 : 0.4

                        StatusIcon {
                            anchors.centerIn: parent
                            type: "next"
                            iconSize: 14
                            color: Theme.fg
                        }

                        MouseArea {
                            anchors.fill: parent
                            enabled: MprisManager.available
                            cursorShape: Qt.PointingHandCursor
                            onClicked: MprisManager.next()
                        }
                    }
                }
            }
        }

        // Audio routing row
        Rectangle {
            width: parent.width
            height: 52
            radius: 16
            color: Theme.bg3
            border.width: 1
            border.color: Theme.border

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 14
                spacing: 10

                StatusIcon {
                    Layout.alignment: Qt.AlignVCenter
                    type: "music"
                    iconSize: 18
                    color: Theme.fg
                }

                Column {
                    Layout.fillWidth: true
                    spacing: 1

                    Txt {
                        text: "Sound"
                        color: Theme.fg
                        font.family: Theme.fontName
                        font.pixelSize: 13
                        font.bold: true
                    }

                    Txt {
                        text: "Output & input"
                        color: Theme.fg3
                        font.family: Theme.fontName
                        font.pixelSize: 11
                    }
                }

                StatusIcon {
                    Layout.alignment: Qt.AlignVCenter
                    type: "chevronRight"
                    iconSize: 14
                    color: Theme.fg3
                }
            }

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: StateController.openModal("audio")
            }
        }
        } // end home body (detailPage === "")
    }
}
