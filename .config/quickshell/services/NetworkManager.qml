pragma Singleton
import QtQuick
import Quickshell
import QsNet 1.0
import "."

// Network state + control, in-process through the QsNet backend: WiFi state
// and AP lists come from NetworkManager D-Bus, Bluetooth from BlueZ D-Bus —
// zero forks, no nmcli, no bluetoothctl (the old Process blocks that forked
// those CLIs are gone). WiFi list and BT device lists are fetched on demand
// (refreshWifi / refreshBluetooth) so nothing polls in the background.
// Connect to saved/known SSIDs directly; unknown networks open a password
// prompt in the control center (see PasswordDialog in ControlPanel.qml).
//
// QML gotcha: ListViews don't observe property changes on plain-JS array
// elements, so every mutation below REASSIGNS the whole array (one notify per
// change) instead of editing an item in place.
Scope {
    id: root

    property bool wifiEnabled: false
    property bool bluetoothEnabled: false
    property bool scanning: false
    property var wifiNetworks: []
    property var btDevices: []
    property bool btScanning: false
    property var currentWifi: ({})

    // Lightweight reads for the expanded bar's status strip: signal strength
    // of the active network and the name of the currently connected BT
    // device. Fetched on demand (bar expand / chip right-click) — never on a
    // timer, so a collapsed bar costs zero traffic.
    property int wifiSignal: -1
    property string btConnectedName: ""

    // Set while a local toggle is in flight so the PropertiesChanged sync can
    // rescan only when OUR flip actually landed (mirrors the old onExited
    // flow; startup reads never trigger scans).
    property bool _togglingWifi: false
    property bool _togglingBt: false

    signal promptPassword(string ssid)
    signal signalError(string message)

    // QsNet keeps truth mirrors of daemon state via D-Bus PropertiesChanged.
    Connections {
        target: QsNet

        function onWifiEnabledChanged() {
			root.wifiEnabled = QsNet.wifiEnabled
			// When wifi goes off, the active network is gone — clear it so a
			// stale row can't linger (old flow refreshed this after toggling).
			if (!root.wifiEnabled)
				root.currentWifi = {}
			if (root._togglingWifi) {
                root._togglingWifi = false
                if (root.wifiEnabled) {
                    root.refreshWifi()
                    root.refreshSignal()
                }
            }
        }
        function onBluetoothEnabledChanged() {
            root.bluetoothEnabled = QsNet.bluetoothEnabled
            if (root._togglingBt) {
                root._togglingBt = false
                if (root.bluetoothEnabled)
                    root.refreshBluetooth()
            }
        }
    }

    function refreshSignal() {
        // Reset first so a stale value can't linger when nothing is connected
        // (e.g. wifi dropped while the bar was collapsed).
        root.wifiSignal = -1
        QsNet.wifiSignal(function(res) {
            if (res && res.name) {
                root.wifiSignal = res.signal
                root.currentWifi = { name: res.name, uuid: res.uuid || "" }
            }
        })
    }

    function refreshBtConnected() {
        // Reset first so a disconnected device's name can't linger.
        root.btConnectedName = ""
        QsNet.btConnectedName(function(name) {
            root.btConnectedName = String(name || "")
        })
    }

    Component.onCompleted: {
        QsNet.refreshWifiEnabled()
        QsNet.refreshBluetoothEnabled()
        root.refreshSignal()
        root.refreshBtConnected()
    }

    function toggleWifi() {
        root._togglingWifi = true
        QsNet.toggleWifi()
    }

    function toggleBluetooth() {
        root._togglingBt = true
        QsNet.toggleBluetooth()
    }

    function refreshWifi() {
        if (root.scanning)
            return
        root.scanning = true
        root.wifiNetworks = []
        QsNet.scanWifi(function(list) {
            const nets = []
            for (let i = 0; i < list.length; i++) {
                nets.push({
                    name: list[i].name,
                    signal: list[i].signal,
                    secured: list[i].secured,
                    // Heuristic parity with the old nmcli mapping: any secured
                    // network is assumed to have a saved profile until a
                    // connect attempt proves otherwise.
                    saved: list[i].secured,
                    active: list[i].active,
                    connecting: false
                })
            }
            root.wifiNetworks = nets
            root.scanning = false
            root.refreshSignal()
        })
    }

    function refreshBluetooth() {
        if (root.btScanning)
            return
        root.btScanning = true
        root.btDevices = []
        QsNet.scanBluetooth(function(list) {
            root.btDevices = list
            root.btScanning = false
        })
    }

    function patchNetwork(ssid, patch) {
        const idx = root.networkIndex(ssid)
        if (idx === -1)
            return
        const next = root.wifiNetworks.slice()
        next[idx] = Object.assign({}, next[idx], patch)
        root.wifiNetworks = next
    }

    function networkIndex(ssid) {
        for (let i = 0; i < root.wifiNetworks.length; i++)
            if (root.wifiNetworks[i].name === ssid)
                return i
        return -1
    }

    function networkBySsid(ssid) {
        const i = root.networkIndex(ssid)
        return i === -1 ? undefined : root.wifiNetworks[i]
    }

    function deviceByAddress(address) {
        for (let i = 0; i < root.btDevices.length; i++)
            if (root.btDevices[i].address === address)
                return root.btDevices[i]
        return undefined
    }

    // Connects directly when the network is saved or has no security.
    // Otherwise asks the control center for a password via the signal.
    function connectWifi(ssid) {
        const net = root.networkBySsid(ssid)
        if (!net)
            return
        if (net.secured && net.saved === false) {
            root.promptPassword(ssid)
            return
        }
        root.patchNetwork(ssid, { connecting: true })
        QsNet.connectWifi(ssid, "", function(res) {
            root.patchNetwork(ssid, { connecting: false })
            if (!res.ok)
                root.signalError("Could not connect to \"" + ssid + "\". " + (res.error || "Try again."))
            root.refreshWifi()
        })
    }

    // Connects to an unknown network with a supplied password.
    function connectWifiPassword(ssid, password) {
        root.patchNetwork(ssid, { connecting: true })
        QsNet.connectWifi(ssid, password, function(res) {
            root.patchNetwork(ssid, { connecting: false })
            if (!res.ok)
                root.signalError("Could not connect to \"" + ssid + "\". " + (res.error || "Try again."))
            root.refreshWifi()
        })
    }

    function disconnectWifi() {
        QsNet.disconnectWifi(function(res) {
            root.currentWifi = {}
            root.refreshWifi()
        })
    }

    function connectBluetooth(address) {
        const idx = root.btDevices.findIndex(function(d) { return d.address === address })
        if (idx !== -1) {
            const next = root.btDevices.slice()
            next[idx] = Object.assign({}, next[idx], { connecting: true })
            root.btDevices = next
        }
        QsNet.connectBluetooth(address, function(res) {
            root.refreshBluetooth()
        })
    }

    function disconnectBluetooth(address) {
        QsNet.disconnectBluetooth(address, function(res) {
            root.refreshBluetooth()
        })
    }
}
