pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Services.UPower
import "."

// Event-driven power state backed by the UPower daemon service (no sysfs
// polling, no Process spawns). AC/percentage/state react instantly to UPower
// property changes; the wattage figure is only sampled once per minute
// (aligned to the minute boundary). Drives the AC plug/unplug OSD toast via
// `acStateChanged` (see StateController.osdPower).
//
// Emits once per transition (baseline is captured on first update, so there
// is no toast at startup) and always keeps the latest values available for
// other components (pill battery, control center power tile, ...).
Scope {
    id: root

    readonly property var battery: UPower.displayDevice

    property bool ready: false
    property bool acOnline: !UPower.onBattery
    property bool charging: battery && battery.state === UPowerDeviceState.Charging
    property bool hasBattery: battery ? battery.isLaptopBattery : false
    // UPower reports percentage as a 0.0-1.0 fraction (Quickshell divides the
    // DBus 0-100 value by 100), so scale before rounding: 0.8 -> 80, not 1.
    property int batteryPercent: battery ? Math.round(battery.percentage * 100) : 0
    property string batteryStatus: statusText()
    // Sampled draw in watts (positive draw; UPower changeRate is negative
    // while discharging). Refreshed at most once per minute.
    property real batteryWatts: 0

    // Remaining run/charge time from UPower (seconds, event-driven — no
    // polling). 0 = unknown (AC desktop, full battery, or UPower idle
    // reporting), which hides the pill chip via timeRemainingText.
    property int timeToEmptySec: battery ? Math.round(battery.timeToEmpty) : 0
    property int timeToFullSec: battery ? Math.round(battery.timeToFull) : 0
    property string timeRemainingText: root.timeRemaining()

    // "2h 13m" while discharging (time to empty), "1h 40m" while charging
    // (time to full), "" when unknown so consumers hide the chip.
    function timeRemaining() {
        const sec = root.charging ? root.timeToFullSec : root.timeToEmptySec
        if (!isFinite(sec) || sec <= 0)
            return ""
        const h = Math.floor(sec / 3600)
        // Clamp so rounding can't roll over to "2h 60m".
        const m = Math.min(59, Math.round((sec % 3600) / 60))
        if (h > 0)
            return h + "h " + m + "m"
        return m + "m"
    }

    // Pre-computed OSD toast content (consumed by the osdpower morph mode).
    // Fully reactive so it is always correct even when hasBattery/batteryStatus
    // settle after startup.
    property string powerIcon: acOnline ? "\uf0e7" : batteryGlyph(batteryPercent)
    property string powerTitle: acOnline
        ? (hasBattery
            ? (batteryStatus === "Charging" ? "Charging"
               : batteryStatus === "Full" ? "Fully Charged"
               : "AC Plugged")
            : "AC Connected")
        : (hasBattery ? "On Battery" : "AC Disconnected")
    property string powerSubtitle: hasBattery && batteryPercent > 0 ? batteryPercent + "%" : ""

    signal acStateChanged()
    signal batteryStateChanged()

    property var prevAc: null
    property var prevPct: null
    property var prevSt: null

    // UPower device state -> shell status string (same vocabulary as the old
    // sysfs reader so consumers keep working).
    function statusText() {
        if (!battery)
            return ""
        switch (battery.state) {
        case UPowerDeviceState.Charging:
            return "Charging"
        case UPowerDeviceState.Discharging:
            return "Discharging"
        case UPowerDeviceState.FullyCharged:
            return "Full"
        case UPowerDeviceState.PendingCharge:
            return "PendingCharge"
        case UPowerDeviceState.PendingDischarge:
            return "PendingDischarge"
        case UPowerDeviceState.Empty:
            return "Empty"
        default:
            return ""
        }
    }

    // FA battery glyphs, level-based (matches Pill's 10-step mapping loosely).
    function batteryGlyph(pct) {
        if (pct >= 90) return "\uf240"
        if (pct >= 70) return "\uf241"
        if (pct >= 45) return "\uf242"
        if (pct >= 20) return "\uf243"
        return "\uf244"
    }

    function refresh() {
        root.batteryWatts = battery ? Math.abs(battery.changeRate) : 0
    }

    // Transition detection: emit only on real AC/battery state changes.
    onAcOnlineChanged: {
        root.ready = true
        if (root.prevAc !== null && acOnline !== root.prevAc)
            root.acStateChanged()
        root.prevAc = acOnline
    }

    onBatteryPercentChanged: {
        root.ready = true
        if (root.prevPct !== null && batteryPercent !== root.prevPct)
            root.batteryStateChanged()
        root.prevPct = batteryPercent
    }

    onBatteryStatusChanged: {
        root.ready = true
        if (root.prevSt !== null && batteryStatus !== root.prevSt)
            root.batteryStateChanged()
        root.prevSt = batteryStatus
    }

    // Sample watts once per minute (60s repeat; fires once at startup).
    Timer {
        id: wattsTicker
        interval: 60000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: root.refresh()
    }
}