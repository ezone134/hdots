import QtQuick
import QtQuick.Shapes
import Qt5Compat.GraphicalEffects
import Quickshell
import Quickshell.Services.UPower
import QsIo 1.0
import "."
import "../components"
import "../services"

Item {
    id: root
    // The expanded pill shrink-wraps its content instead of being a fixed
    // 520px: row 1 splits into two zones — the context cluster (current app /
    // window) grows from the left edge, and every module chip (tray, battery,
    // settings, power …) is anchored to the right edge. leftClusterWidth feeds
    // the width calc for both states. (The clock/date cluster is dead-centered
    // when expanded, so it no longer contributes to this calc.)
    property real leftClusterWidth: contextCluster.implicitWidth
    // The expanded pill grows three bands: row 1 (two-zone chips), the status
    // strip (hero clock dead-center above the audio/power status), and the
    // info row (now-playing left, calendar strip + weather right) pinned to
    // the bottom. Collapsed stays the classic 38px clock-only pill.
    // Silhouette: capsule while floating, flat bar while stuck.
    property real expandedHeight: 256
    property real infoWidth: 800
    // Both clusters grow outward from their anchors: the left cluster flows
    // from the left edge, the module cluster is right-anchored, so the bar
    // tracks the sum of both sides. infoWidth stays the floor.
    property real expandedWidth: Math.max(520, root.leftClusterWidth + 18 + statusRow.implicitWidth + 24, root.infoWidth)
    implicitWidth: root.isExpanded ? root.expandedWidth : root.collapsedWidth
    implicitHeight: root.isExpanded ? root.expandedHeight : 38
    property color fgColor: "#fcfcfc"
    property bool showContents: false

    // True while the mouse is over the bar during collapse mode (expanded
    // bar off). externalHover is driven by ShellRoot.qml's full-window hover
    // (the whole bar window is the hover target, so the island expands
    // when the cursor is over the rect — not just the clock text).
    property bool externalHover: false
    property bool hovered: false
    // expandOnHover (Settings → Shell): dashboard while cursor is over the
    // island. Off = simple compact bar only (no always-expanded mode).
    property bool expandOnHover: SettingsManager.config.shell.expandOnHover !== false
    property bool isExpanded: root.expandOnHover && (root.hovered || root.externalHover)
    // Report the momentary expanded state to services: WeatherManager's
    // 10-min timer polls only while the expanded bar is actually shown
    // (collapsed-by-default mode expands on hover). Assignment-in-binding is
    // the codebase's "touch a singleton via a property binding" convention.
    property bool expandedReport: StateController.barExpanded = root.isExpanded
    property bool stuckTop: SettingsManager.config.shell.floating === false
    property real slideDur: 250
    property real fadeDur: 150
    property real scaleDur: 150

    property bool hoverCollapse: root.expandOnHover

    // Matches the container's expand/collapse timing so the bar
    // grows in lockstep with the surface — no clipping seams. The
    // pill is never scaled (MorphContainer.fitScale skips pill
    // mode), so the clock text renders directly and never shimmers.
    property real hoverWidth: root.isExpanded ? root.expandedWidth : root.collapsedWidth

    // Live clock: only ticks once a minute (60s repeat) instead of every
    // second; fires once at startup so the display is correct immediately.
    // The 60s tick is free-running (not aligned to the wall-clock minute),
    // which is fine since the label only needs minute resolution.
    // Text binds to root.now (font.pixelSize is constant, so glyphs are
    // never re-rasterized every frame).
    property date now: new Date()
    Timer {
        id: clockTicker
        interval: 60000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: root.now = new Date()
    }

    Component.onCompleted: {
        // Always-expanded mode never flips isExpanded after startup, so the
        // onIsExpandedChanged refresh below would never fire — pull the
        // connectivity reads once per pill creation too (one-shot processes,
        // same on-demand philosophy as the control center's open-refresh).
        if (root.isExpanded) {
            NetworkManager.refreshSignal()
            NetworkManager.refreshBtConnected()
        }
    }

    // 3s net up/down sampler. Gated on the expanded bar AND the pill being
    // the visible mode, so a collapsed bar costs zero work and modal overlays
    // (launcher/control/calendar …) don't sample for an invisible chip — same
    // gating philosophy as the WeatherManager 10-min timer. Reads
    // /proc/net/dev IN-PROCESS via QsIo.readFile — no fork, no bash. This
    // killed the last timer-driven Process in the shell; sampleNet() keeps the
    // exact byte counters + dt=3 rate math the old bash sampler had.
    Timer {
        id: netTicker
        interval: 3000
        repeat: true
        triggeredOnStart: true
        running: root.isExpanded && StateController.osdMode === "pill"
        onTriggered: root.sampleNet()
    }

    // Picks the non-loopback interface with the most traffic and updates the
    // net up/down labels. Same algorithm as the old bash one-liner, ported to
    // JS on top of QsIo.readFile (zero forks). (NB: /proc/net/dev data lines
    // have no '|' separator — it's only in the header — so fields are plain
    // space-separated: index 0 = rx_bytes, index 8 = tx_bytes.)
    function sampleNet() {
        const raw = QsIo.readFile("/proc/net/dev")
        const lines = String(raw || "").split("\n")
        let brx = -1
        let name = ""
        let rxv = 0
        let txv = 0
        for (let i = 2; i < lines.length; i++) {
            const ci = lines[i].indexOf(":")
            if (ci === -1)
                continue
            const iface = lines[i].slice(0, ci).trim()
            if (iface === "lo" || iface.length === 0)
                continue
            const parts = lines[i].slice(ci + 1).trim().split(/\s+/)
            const rx = parseInt(parts[0], 10) || 0
            const tx = parseInt(parts[8], 10) || 0
            const tot = rx + tx
            if (tot > brx) {
                brx = tot
                name = iface
                rxv = rx
                txv = tx
            }
        }
        if (name.length === 0)
            return
        if (root.prevRx >= 0 && root.prevTx >= 0) {
            const dt = 3
            root.netUpText = root.fmtRate(Math.max(0, (txv - root.prevTx)) / dt)
            root.netDownText = root.fmtRate(Math.max(0, (rxv - root.prevRx)) / dt)
        }
        root.prevRx = rxv
        root.prevTx = txv
    }

    // Battery/watts come from the PowerManager singleton (UPower service).
    // The drawn BatteryIcon (ported from Tide Island) shows a real battery
    // shape: animated fill with the level inside, a bolt while charging and
    // a red body under 20%. battCharging mirrors the old glyph logic
    // (plugged-in -> charging style); battFill picks the body color.
    property int battPerc: PowerManager.hasBattery ? PowerManager.batteryPercent : 0
    property bool battCharging: PowerManager.hasBattery && PowerManager.acOnline
    property color battFill: root.battCharging || root.battPerc > 20
        ? (Theme.mode === "light" ? Theme.fg : "#ffffff")
        : "#ff3b30"
    property real instantWatts: PowerManager.hasBattery ? PowerManager.batteryWatts : 0

    // Collapsed-by-default bar: which status items stay visible next to the
    // clock when the bar is NOT expanded (settings app -> Pill). All off
    // keeps the classic clock-only 135px island. Tray items are never shown
    // collapsed.
    property bool collapsedBattery: SettingsManager.config.pill && SettingsManager.config.pill.collapsedBattery === true
    property bool collapsedWatts: SettingsManager.config.pill && SettingsManager.config.pill.collapsedWatts === true
    property bool collapsedWeather: SettingsManager.config.pill && SettingsManager.config.pill.collapsedWeather === true
    property bool collapsedNotification: SettingsManager.config.pill && SettingsManager.config.pill.collapsedNotification === true
    property bool collapsedWorkspace: SettingsManager.config.pill && SettingsManager.config.pill.collapsedWorkspace === true
    property bool collapsedSettings: SettingsManager.config.pill && SettingsManager.config.pill.collapsedSettings === true
    property bool collapsedPower: SettingsManager.config.pill && SettingsManager.config.pill.collapsedPower === true
    // Collapsed EQ only (default on). Expanded always animates while Playing.
    property bool musicBarAnim: !(SettingsManager.config.pill && SettingsManager.config.pill.musicBarAnim === false)
    property bool mediaPlaying: MprisManager.status === "Playing"
    property bool collapsedMusicEq: root.musicBarAnim && root.mediaPlaying && !root.isExpanded
    property bool expandedMusicEq: root.mediaPlaying && root.isExpanded
    property bool collapsedHasItems: root.collapsedBattery || root.collapsedWatts || root.collapsedWeather ||
        root.collapsedNotification || root.collapsedWorkspace || root.collapsedSettings ||
        root.collapsedPower || root.collapsedMusicEq

    readonly property var defaultPillOrder: [
        "clock", "musicBarAnim", "collapsedBattery", "collapsedWatts", "collapsedWeather",
        "collapsedNotification", "collapsedWorkspace", "collapsedSettings", "collapsedPower"
    ]

    // Settings → Pill order; clock is always included (reorderable, never off).
    property var collapsedItemOrder: {
        const raw = (SettingsManager.config.pill && SettingsManager.config.pill.itemOrder) || []
        const out = []
        const seen = {}
        for (let i = 0; i < raw.length; i++) {
            const k = String(raw[i] || "")
            if (!k || seen[k])
                continue
            if (root.defaultPillOrder.indexOf(k) < 0)
                continue
            seen[k] = true
            out.push(k)
        }
        for (let j = 0; j < root.defaultPillOrder.length; j++) {
            const d = root.defaultPillOrder[j]
            if (!seen[d])
                out.push(d)
        }
        if (out.indexOf("clock") < 0)
            out.unshift("clock")
        return out
    }

    // Collapsed chip factories (Loader sourceComponent). Each root exposes
    // showChip + chipWidth so the Row can collapse hidden slots to 0 width.
    property Component collapsedClockComp: Component {
        Rectangle {
            id: clockChipInner
            readonly property bool showChip: true
            // Match other chips: content width only; Row spacing owns gaps.
            readonly property real chipWidth: clockLabelInner.implicitWidth + 10
            implicitWidth: chipWidth
            width: chipWidth
            height: 28
            radius: 14
            color: clockChipMa.containsMouse ? Qt.rgba(0, 0, 0, 0.12) : Qt.rgba(0, 0, 0, 0.06)
            visible: showChip

            Behavior on color {
                NumberAnimation { duration: 150 }
            }

            Txt {
                id: clockLabelInner
                anchors.centerIn: parent
                text: Qt.formatDateTime(root.now, "hh:mm")
                color: Theme.fg
                font.bold: true
                font.pixelSize: 20
            }

            MouseArea {
                id: clockChipMa
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: function(mouse) {
                    mouse.accepted = true
                    StateController.calendar()
                }
            }
        }
    }

    property Component collapsedEqComp: Component {
        Item {
            id: eqChip
            readonly property bool showChip: root.musicBarAnim && MprisManager.status === "Playing"
            readonly property real chipWidth: showChip ? eqBars.implicitWidth : 0
            implicitWidth: chipWidth
            implicitHeight: 16
            width: chipWidth
            height: 16
            visible: showChip

            Row {
                id: eqBars
                anchors.centerIn: parent
                spacing: 2.5
                height: 14

                Repeater {
                    model: 4
                    delegate: Rectangle {
                        required property int index
                        width: 2.5
                        radius: 1
                        anchors.bottom: parent ? parent.bottom : undefined
                        color: Theme.acc
                        height: 4

                        SequentialAnimation on height {
                            loops: Animation.Infinite
                            running: root.collapsedMusicEq && eqChip.showChip
                            PauseAnimation { duration: index * 70 }
                            NumberAnimation {
                                from: 4
                                to: 14
                                duration: 260 + index * 35
                                easing.type: Easing.InOutSine
                            }
                            NumberAnimation {
                                from: 14
                                to: 4
                                duration: 260 + index * 35
                                easing.type: Easing.InOutSine
                            }
                        }
                    }
                }
            }

            MouseArea {
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: function(mouse) {
                    mouse.accepted = true
                    StateController.mpris()
                }
            }
        }
    }

    property Component collapsedBattComp: Component {
        Item {
            readonly property bool showChip: root.collapsedBattery && PowerManager.hasBattery
            readonly property real chipWidth: showChip ? battRow.implicitWidth : 0
            implicitWidth: chipWidth
            width: chipWidth
            height: 18
            visible: showChip

            Row {
                id: battRow
                anchors.verticalCenter: parent.verticalCenter
                BatteryIcon {
                    anchors.verticalCenter: parent.verticalCenter
                    level: root.battPerc
                    charging: root.battCharging
                    fillColor: root.battFill
                    fontFamily: Theme.fontName
                    iconFontFamily: Theme.iconFont
                    fontSize: 12
                    chargingFontSize: 11
                }
            }
        }
    }

    property Component collapsedWattsComp: Component {
        Item {
            readonly property bool showChip: root.collapsedWatts && PowerManager.hasBattery
            readonly property real chipWidth: showChip ? wattsTxt.implicitWidth : 0
            implicitWidth: chipWidth
            width: chipWidth
            height: 18
            visible: showChip

            Txt {
                id: wattsTxt
                anchors.verticalCenter: parent.verticalCenter
                text: root.instantWatts.toFixed(1) + " W"
                font.family: Theme.fontName
                font.pixelSize: 13
                color: Theme.fg3
            }
        }
    }

    property Component collapsedWeatherComp: Component {
        Item {
            readonly property bool showChip: root.collapsedWeather
            readonly property real chipWidth: showChip ? (wGlyphTxt.implicitWidth + 5 + wTempTxt.implicitWidth) : 0
            implicitWidth: chipWidth
            width: chipWidth
            height: 18
            visible: showChip

            Txt {
                id: wGlyphTxt
                anchors.verticalCenter: parent.verticalCenter
                text: WeatherManager.glyph.length > 0 ? WeatherManager.glyph : "\uf0c2"
                font.family: Theme.iconFont
                font.pixelSize: 13
                color: Theme.fg
            }

            Txt {
                id: wTempTxt
                anchors.left: wGlyphTxt.right
                anchors.leftMargin: 5
                anchors.verticalCenter: parent.verticalCenter
                text: WeatherManager.ready ? WeatherManager.temperature + "°" : ""
                font.family: Theme.fontName
                font.pixelSize: 12
                font.bold: true
                color: Theme.fg3
            }

            MouseArea {
                anchors.fill: parent
                hoverEnabled: true
                acceptedButtons: Qt.LeftButton | Qt.RightButton
                cursorShape: Qt.PointingHandCursor
                onClicked: function(mouse) {
                    mouse.accepted = true
                    if (mouse.button === Qt.RightButton)
                        WeatherManager.refresh()
                    else
                        StateController.weather()
                }
            }
        }
    }

    property Component collapsedNotifComp: Component {
        Item {
            readonly property bool showChip: root.collapsedNotification
            readonly property real chipWidth: showChip ? 16 : 0
            implicitWidth: chipWidth
            width: chipWidth
            height: 18
            visible: showChip

            StatusIcon {
                anchors.centerIn: parent
                type: "bell"
                iconSize: 14
                color: Theme.fg
            }
            MouseArea {
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    mouse.accepted = true
                    NotificationManager.markAllRead()
                    StateController.notificationCenter()
                }
            }
        }
    }

    property Component collapsedWsComp: Component {
        Item {
            readonly property bool showChip: root.collapsedWorkspace
            readonly property real chipWidth: showChip ? 16 : 0
            implicitWidth: chipWidth
            width: chipWidth
            height: 18
            visible: showChip

            StatusIcon {
                anchors.centerIn: parent
                type: "grid"
                iconSize: 14
                color: Theme.fg
            }
            MouseArea {
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    mouse.accepted = true
                    StateController.workspaceSwitcher()
                }
            }
        }
    }

    property Component collapsedSettingsComp: Component {
        Item {
            readonly property bool showChip: root.collapsedSettings
            readonly property real chipWidth: showChip ? 16 : 0
            implicitWidth: chipWidth
            width: chipWidth
            height: 18
            visible: showChip

            StatusIcon {
                anchors.centerIn: parent
                type: "gear"
                iconSize: 14
                color: Theme.fg
            }
            MouseArea {
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    mouse.accepted = true
                    StateController.settings()
                }
            }
        }
    }

    property Component collapsedPowerComp: Component {
        Item {
            readonly property bool showChip: root.collapsedPower
            readonly property real chipWidth: showChip ? 16 : 0
            implicitWidth: chipWidth
            width: chipWidth
            height: 18
            visible: showChip

            StatusIcon {
                anchors.centerIn: parent
                type: "power"
                iconSize: 14
                color: Theme.fg
            }
            MouseArea {
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    mouse.accepted = true
                    StateController.powerMenu()
                }
            }
        }
    }

    // Collapsed island: ordered row always includes clock. Width tracks the
    // row (+ margins), floored at the classic 135px size.
    property real collapsedWidth: Math.max(135, collapsedItemsRow.implicitWidth + 32)

    // Net up/down sampler state: byte counters from /proc/net/dev sampled
    // every 3s while expanded (netTicker below); deltas become per-second
    // labels. prevRx/prevTx are -1 until a baseline sample lands.
    property string netUpText: "--"
    property string netDownText: "--"
    property double prevRx: -1
    property double prevTx: -1

    // Bytes/sec -> compact label: "1.2M", "340K", "12B".
    function fmtRate(bps) {
        if (bps >= 1048576)
            return (bps / 1048576).toFixed(1) + "M"
        if (bps >= 1024)
            return Math.round(bps / 1024) + "K"
        return Math.round(bps) + "B"
    }

    // Focused-window indicator (macOS "frontmost app" block, left of the
    // middle status strip): app glyph tile + bold app name + elided window
    // title. Falls back to the workspace label when no window is focused.
    // Fully reactive through HyprlandManager.focusedWindow — no timers.
    property var focusWin: HyprlandManager.focusedWindow
    property bool focusHasWindow: !!root.focusWin
    property string focusGlyph: root.focusWin ? HyprlandManager.appGlyph(root.focusWin.className) : "󰍺"
    property string focusAppName: root.focusWin ? root.appName(root.focusWin.className) : "Desktop"
    property string focusWindowTitle: root.focusWin ? String(root.focusWin.title) : ""
    property string focusDesktopLabel: root.focusHasWindow ? "" : (String(HyprlandManager.workspaceNames[String(HyprlandManager.currentWorkspace)] || "") || ("Workspace " + HyprlandManager.currentWorkspace))

    // Compact m:ss readout for the now-playing progress bar.
    function fmtTime(sec) {
        if (!isFinite(sec) || sec < 0)
            sec = 0
        const m = Math.floor(sec / 60)
        const s = Math.floor(sec % 60)
        return m + ":" + String(s).padStart(2, "0")
    }

    // Display name from a Hyprland class: last dotted segment, title-cased
    // ("org.kde.dolphin" -> "Dolphin", "code" -> "Code").
    function appName(cls) {
        const raw = String(cls || "").trim()
        if (raw.length === 0)
            return "App"
        const seg = raw.split(".").pop()
        return seg.charAt(0).toUpperCase() + seg.slice(1)
    }


    // On expand: pull fresh wifi signal + connected BT device for the status
    // strip (both are on-demand reads, never timers) and reset the net-speed
    // baseline so the first tick after a long collapse can't report a huge
    // delta. netTicker itself is gated on isExpanded, so a collapsed bar
    // still runs zero processes.
    onIsExpandedChanged: {
        if (root.isExpanded) {
            NetworkManager.refreshSignal()
            NetworkManager.refreshBtConnected()
            root.prevRx = -1
            root.prevTx = -1
        }
    }

    // Five-day strip for the expanded info row: today-2 … today+2 with the
    // weekday abbreviation above each cell (today highlighted, like the
    // reference layout). Zero-padded day numbers, e.g. 06 07 08 09 10.
    function dayCells() {
        const t = root.now
        const out = []
        for (let i = -2; i <= 2; i++) {
            const d = new Date(t.getFullYear(), t.getMonth(), t.getDate() + i)
            out.push({
                day: Qt.formatDate(d, "dd"),
                wd: Qt.formatDate(d, "ddd").substring(0, 2).toUpperCase(),
                isToday: i === 0
            })
        }
        return out
    }

    Item {
        id: hoverArea
        anchors.fill: parent

        Shape {
            id: pill
            anchors.centerIn: parent
            width: root.isExpanded ? root.expandedWidth : root.collapsedWidth
            height: root.isExpanded ? root.expandedHeight : 38
            layer.enabled: true
            layer.samples: 2
            layer.effect: OpacityMask {
                maskSource: pillMask
            }
            antialiasing: true

            // Silhouette: floating = a true capsule pill (radius = half the
            // height on every corner); stuck-to-top (floating off) = a flat
            // bar flush at the screen top with small rounded top corners and
            // a straight bottom edge. No U-shaped bottom anywhere. The
            // Behaviors smooth the corner jump when the mode or the
            // floating toggle flips.
            property real topR: root.stuckTop ? 8 : height / 2
            property real botR: root.stuckTop ? 0 : height / 2

            Behavior on topR {
                NumberAnimation { duration: 150; easing.type: Easing.OutCubic }
            }
            Behavior on botR {
                NumberAnimation { duration: 150; easing.type: Easing.OutCubic }
            }

            ShapePath {
                strokeWidth: 0
                fillColor: Theme.bg
                strokeColor: "transparent"

                startX: pill.topR
                startY: 0

                // Top edge
                PathLine {
                    x: pill.width - pill.topR
                    y: 0
                }

                // Top-right corner — rounded
                PathQuad {
                    x: pill.width
                    y: pill.topR
                    controlX: pill.width
                    controlY: 0
                }

                // Right side
                PathLine {
                    x: pill.width
                    y: pill.height - pill.botR
                }

                // Bottom-right — deep U curve
                PathQuad {
                    x: pill.width - pill.botR
                    y: pill.height
                    controlX: pill.width
                    controlY: pill.height
                }

                // Bottom edge
                PathLine {
                    x: pill.botR
                    y: pill.height
                }

                // Bottom-left — deep U curve
                PathQuad {
                    x: 0
                    y: pill.height - pill.botR
                    controlX: 0
                    controlY: pill.height
                }

                // Left side
                PathLine {
                    x: 0
                    y: pill.topR
                }

                // Top-left corner — rounded
                PathQuad {
                    x: pill.topR
                    y: 0
                    controlX: 0
                    controlY: 0
                }
            }



            // Matches the container's expand/collapse timing so the bar
            // grows in lockstep with the surface — no clipping seams. The
            // pill is never scaled (MorphContainer.fitScale skips pill
            // mode), so the clock text renders directly and never shimmers.
            Behavior on width {
                NumberAnimation {
                    duration: 240
                    easing.type: Easing.OutCubic
                }
            }

            Behavior on height {
                NumberAnimation {
                    duration: root.slideDur - 100
                    easing.type: Easing.OutCubic
                }
            }

            // Invisible anchor band for row 1 (the chip row): the clock,
            // tray, battery, settings and power cluster stays in a 38px band
            // at the top while the pill is expanded into two rows, and
            // dead-center while collapsed (y=0). The chips' verticalCenter
            // anchors resolve to this band's center line.
            Item {
                id: row1Anchor
                width: 1
                height: 38
                y: root.isExpanded ? 6 : 0
                visible: false

                Behavior on y {
                    NumberAnimation {
                        duration: 240
                        easing.type: Easing.OutCubic
                    }
                }
            }

            // Anchor helpers (invisible)
            Txt {
                id: dummyL
                text: ""
                color: "white"
                font.pixelSize: 16
                font.bold: true
                anchors.left: parent.left
                opacity: root.isExpanded ? 1 : 0
                Behavior on opacity {
                    NumberAnimation { duration: 200 }
                }
            }

            Txt {
                id: dummyR
                text: ""
                color: "white"
                font.pixelSize: 16
                font.bold: true
                anchors.right: parent.right
                opacity: root.isExpanded ? 1 : 0
                Behavior on opacity {
                    NumberAnimation { duration: 200 }
                }
            }

            Item {
                id: contextCluster
                // Current app / window context: glyph tile + app name +
                // window title. Click = window switcher (workspace
                // switcher with no window), wheel = cycle windows.
                height: 26
                width: Math.min(280, contextRow.implicitWidth + 34)
                anchors.left: dummyL.left
                anchors.leftMargin: 10
                anchors.verticalCenter: row1Anchor.verticalCenter
                opacity: root.isExpanded ? 1 : 0
                scale: root.isExpanded ? 1.0 : 0.85
                transformOrigin: Item.Center

                Behavior on opacity {
                    NumberAnimation {
                        duration: root.fadeDur
                        easing.type: Easing.OutQuad
                    }
                }
                Behavior on scale {
                    NumberAnimation {
                        duration: root.scaleDur
                        easing.type: Easing.OutCubic
                    }
                }

                Rectangle {
                    id: contextTile
                    width: 26
                    height: 26
                    radius: 13
                    color: contextHover.containsMouse ? Theme.acc : Theme.bg3
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter

                    Behavior on color {
                        ColorAnimation { duration: 120 }
                    }

                    Txt {
                        anchors.centerIn: parent
                        text: root.focusGlyph
                        font.family: Theme.iconFont
                        font.pixelSize: 12
                        color: contextHover.containsMouse ? Theme.sfg : Theme.fg
                    }
                }

                Row {
                    id: contextRow
                    anchors.left: contextTile.right
                    anchors.leftMargin: 8
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 4

                    Txt {
                        id: contextName
                        text: root.focusAppName
                        color: Theme.fg
                        font.family: Theme.fontName
                        font.pixelSize: 12
                        font.bold: true
                    }

                    Txt {
                        visible: root.focusHasWindow
                        text: "\u00b7"
                        color: Theme.fg3
                        font.family: Theme.fontName
                        font.pixelSize: 12
                    }

                    Txt {
                        id: contextTitle
                        visible: root.focusHasWindow
                        text: root.focusWindowTitle
                        color: Theme.fg2
                        font.family: Theme.fontName
                        font.pixelSize: 12
                        elide: Text.ElideRight
                        width: Math.min(150, 240 - contextName.implicitWidth - 24)
                    }
                }

                MouseArea {
                    id: contextHover
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        mouse.accepted = true
                        if (root.focusHasWindow)
                            StateController.windowSwitcher()
                        else
                            StateController.workspaceSwitcher()
                    }
                    onWheel: function(wheel) {
                        wheel.accepted = true
                        if (root.focusHasWindow) {
                            const wins = HyprlandManager.allWindows.filter(function(w) {
                                return w.mapped !== false && w.workspaceId === HyprlandManager.currentWorkspace
                            })
                            if (wins.length === 0)
                                return
                            let idx = 0
                            for (let i = 0; i < wins.length; i++) {
                                if (String(wins[i].address) === String(root.focusWin.address)) {
                                    idx = i
                                    break
                                }
                            }
                            idx = (idx + (wheel.angleDelta.y > 0 ? 1 : wins.length - 1)) % wins.length
                            HyprlandManager.focusWindow(wins[idx].address)
                        } else {
                            const ids = HyprlandManager.workspaceIds
                            if (ids.length === 0)
                                return
                            const ci = ids.indexOf(HyprlandManager.currentWorkspace)
                            let n = ci >= 0 ? (ci + (wheel.angleDelta.y > 0 ? 1 : -1)) : 0
                            n = (n + ids.length) % ids.length
                            HyprlandManager.focusWorkspace(ids[n])
                        }
                    }
                }
            }

            // Collapsed clock lives inside collapsedItemsRow (pill.itemOrder
            // "clock") so it can be reordered. Expanded uses heroClockBlock.

            Row {
                id: statusRow
                spacing: 15
                // Always glued to the pill's right edge (visible only while
                // expanded): the chips fade/scale in place and ride the edge
                // as the bar grows instead of sliding across from the left.
                // No Behavior on x — it tracks the animating pill.width.
                x: pill.width - statusRow.implicitWidth - 16
                anchors.verticalCenter: row1Anchor.verticalCenter

                Tray {
                    enabled: root.isExpanded
                    opacity: root.isExpanded ? 1 : 0
                    scale: root.isExpanded ? 1.0 : 0.85
                    transformOrigin: Item.Center

                    Behavior on opacity {
                        NumberAnimation {
                            duration: root.fadeDur
                            easing.type: Easing.OutQuad
                        }
                    }
                    Behavior on scale {
                        NumberAnimation {
                            duration: root.scaleDur
                            easing.type: Easing.OutCubic
                        }
                    }
                }

                // WiFi: glyph + SSID + signal strength (expanded only). Click =
                // control center, right-click = re-read the signal. The Item
                // wrapper (not a bare Row) keeps the MouseArea out of the Row
                // layout so there are no anchors-in-positioner warnings.
                Item {
                    id: wifiChip
                    anchors.verticalCenter: parent.verticalCenter
                    width: wifiChipRow.implicitWidth
                    height: 24
                    opacity: root.isExpanded ? 1 : 0
                    scale: root.isExpanded ? 1.0 : 0.85
                    transformOrigin: Item.Center

                    Behavior on opacity {
                        NumberAnimation {
                            duration: root.fadeDur
                            easing.type: Easing.OutQuad
                        }
                    }
                    Behavior on scale {
                        NumberAnimation {
                            duration: root.scaleDur
                            easing.type: Easing.OutCubic
                        }
                    }

                    Row {
                        id: wifiChipRow
                        anchors.fill: parent
                        spacing: 5

                        // Drawn wifi icon (shared StatusIcon): the number of
                        // lit arcs tracks signal strength, so the old
                        // glyph + bars Txt are replaced by one component.
                        StatusIcon {
                            anchors.verticalCenter: parent.verticalCenter
                            type: "wifi"
                            level: NetworkManager.wifiSignal / 100
                            active: NetworkManager.wifiEnabled
                            color: Theme.fg
                            iconSize: 20
                        }

                        Txt {
                            // Truthiness (not .length): currentWifi is {} until
                            // the first read lands, and undefined.length throws.
                            // `|| ""` keeps the QString binding clean meanwhile.
                            visible: NetworkManager.wifiEnabled && NetworkManager.currentWifi.name
                            anchors.verticalCenter: parent.verticalCenter
                            text: NetworkManager.currentWifi.name || ""
                            color: Theme.fg
                            font.family: Theme.fontName
                            font.pixelSize: 12
                            elide: Text.ElideRight
                            width: 130
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                        cursorShape: Qt.PointingHandCursor
                        onClicked: function(mouse) {
                            mouse.accepted = true
                            if (mouse.button === Qt.RightButton)
                                NetworkManager.refreshSignal()
                            else
                                StateController.controlCenter()
                        }
                    }
                }

                // Bluetooth: glyph + connected device name (expanded only).
                Item {
                    id: btChip
                    anchors.verticalCenter: parent.verticalCenter
                    width: btChipRow.implicitWidth
                    height: 24
                    opacity: root.isExpanded ? 1 : 0
                    scale: root.isExpanded ? 1.0 : 0.85
                    transformOrigin: Item.Center

                    Behavior on opacity {
                        NumberAnimation {
                            duration: root.fadeDur
                            easing.type: Easing.OutQuad
                        }
                    }
                    Behavior on scale {
                        NumberAnimation {
                            duration: root.scaleDur
                            easing.type: Easing.OutCubic
                        }
                    }

                    Row {
                        id: btChipRow
                        anchors.fill: parent
                        spacing: 5

                        // Drawn bluetooth rune (shared StatusIcon); dims
                        // automatically when the adapter is powered off.
                        StatusIcon {
                            anchors.verticalCenter: parent.verticalCenter
                            type: "bluetooth"
                            active: NetworkManager.bluetoothEnabled
                            color: Theme.fg
                            iconSize: 20
                        }

                        Txt {
                            visible: NetworkManager.btConnectedName.length > 0
                            anchors.verticalCenter: parent.verticalCenter
                            text: NetworkManager.btConnectedName
                            color: Theme.fg
                            font.family: Theme.fontName
                            font.pixelSize: 12
                            elide: Text.ElideRight
                            width: 110
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: function(mouse) {
                            mouse.accepted = true
                            StateController.controlCenter()
                        }
                    }
                }

                Row {
                    visible: PowerManager.hasBattery
                    anchors.verticalCenter: parent.verticalCenter
                    opacity: root.isExpanded ? 1 : 0
                    scale: root.isExpanded ? 1.0 : 0.85
                    transformOrigin: Item.Center

                    Behavior on opacity {
                        NumberAnimation {
                            duration: root.fadeDur
                            easing.type: Easing.OutQuad
                        }
                    }
                    Behavior on scale {
                        NumberAnimation {
                            duration: root.scaleDur
                            easing.type: Easing.OutCubic
                        }
                    }

                    BatteryIcon {
                        anchors.verticalCenter: parent.verticalCenter
                        level: root.battPerc
                        charging: root.battCharging
                        fillColor: root.battFill
                        fontFamily: Theme.fontName
                        iconFontFamily: Theme.iconFont
                    }
                }

                Txt {
                    visible: PowerManager.hasBattery
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.instantWatts.toFixed(1) + " W"
                    color: Theme.fg3
                    font.pixelSize: 14
                    opacity: root.isExpanded ? 1 : 0
                    scale: root.isExpanded ? 1.0 : 0.85
                    transformOrigin: Item.Center

                    Behavior on opacity {
                        NumberAnimation {
                            duration: root.fadeDur
                            easing.type: Easing.OutQuad
                        }
                    }
                    Behavior on scale {
                        NumberAnimation {
                            duration: root.scaleDur
                            easing.type: Easing.OutCubic
                        }
                    }
                }

                // Settings gear (expanded only): opens the settings
                // panel. The control-center chip (gear + unread badge)
                // sits right of it and opens the control center.
                Rectangle {
                    z: 100
                    width: 44
                    height: 26
                    radius: 13
                    color: "transparent"
                    opacity: root.isExpanded ? 1 : 0
                    scale: root.isExpanded ? 1.0 : 0.85
                    transformOrigin: Item.Center

                    Behavior on opacity {
                        NumberAnimation {
                            duration: root.fadeDur
                            easing.type: Easing.OutQuad
                        }
                    }
                    Behavior on scale {
                        NumberAnimation {
                            duration: root.scaleDur
                            easing.type: Easing.OutCubic
                        }
                    }

                    Rectangle {
                        anchors.fill: parent
                        radius: 13
                        color: settingsChipHover.containsMouse ? Theme.hover : Theme.bg3

                        Behavior on color {
                            ColorAnimation { duration: 120 }
                        }

                        StatusIcon {
                            anchors.centerIn: parent
                            type: "gear"
                            iconSize: 16
                            color: settingsChipHover.containsMouse ? Theme.sfg : Theme.fg
                        }
                    }

                    MouseArea {
                        id: settingsChipHover
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: function(mouse) {
                            mouse.accepted = true
                            StateController.settings()
                        }
                    }
                }

                Rectangle {
                    z: 100
                    width: 44
                    height: 26
                    radius: 13
                    color: "transparent"
                    opacity: root.isExpanded ? 1 : 0
                    scale: root.isExpanded ? 1.0 : 0.85
                    transformOrigin: Item.Center

                    Behavior on opacity {
                        NumberAnimation {
                            duration: root.fadeDur
                            easing.type: Easing.OutQuad
                        }
                    }
                    Behavior on scale {
                        NumberAnimation {
                            duration: root.scaleDur
                            easing.type: Easing.OutCubic
                        }
                    }

                    Rectangle {
                        anchors.fill: parent
                        radius: 13
                        color: ctrChipHover.containsMouse ? Theme.hover : Theme.bg3

                        Behavior on color {
                            ColorAnimation { duration: 120 }
                        }

                        StatusIcon {
                            anchors.centerIn: parent
                            type: "sliders"
                            iconSize: 16
                            color: ctrChipHover.containsMouse ? Theme.sfg : Theme.fg
                        }
                    }

                    Rectangle {
                        visible: NotificationManager.unreadCount > 0
                        width: Math.max(14, ctrBadge.implicitWidth + 8)
                        height: 14
                        radius: 7
                        color: Theme.acc
                        anchors.right: parent.right
                        anchors.top: parent.top

                        Txt {
                            id: ctrBadge
                            anchors.centerIn: parent
                            text: NotificationManager.unreadCount > 99 ? "99+" : String(NotificationManager.unreadCount)
                            color: Theme.sfg
                            font.family: Theme.fontName
                            font.pixelSize: 9
                            font.bold: true
                        }
                    }

                    MouseArea {
                        id: ctrChipHover
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: function(mouse) {
                            mouse.accepted = true
                            NotificationManager.markAllRead()
                            StateController.controlCenter()
                        }
                    }
                }

                // Theme toggle (expanded only): flips light/dark. Applies +
                // persists via Theme.toggleMode(), which also fires
                // `theme_main <mode>` for the user's GTK dark-mode hook.
                Rectangle {
                    z: 100
                    width: 44
                    height: 26
                    radius: 13
                    color: "transparent"
                    opacity: root.isExpanded ? 1 : 0
                    scale: root.isExpanded ? 1.0 : 0.85
                    transformOrigin: Item.Center

                    Behavior on opacity {
                        NumberAnimation {
                            duration: root.fadeDur
                            easing.type: Easing.OutQuad
                        }
                    }
                    Behavior on scale {
                        NumberAnimation {
                            duration: root.scaleDur
                            easing.type: Easing.OutCubic
                        }
                    }

                    Rectangle {
                        anchors.fill: parent
                        radius: 13
                        color: themeChipHover.containsMouse ? Theme.hover : Theme.bg3

                        Behavior on color {
                            ColorAnimation { duration: 120 }
                        }

                        StatusIcon {
                            anchors.centerIn: parent
                            type: Theme.mode === "dark" ? "sun" : "moon"
                            iconSize: 16
                            color: Theme.mode === "dark"
                                ? (themeChipHover.containsMouse ? Theme.sfg : Theme.fg)
                                : Theme.fg3
                        }
                    }

                    MouseArea {
                        id: themeChipHover
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: function(mouse) {
                            mouse.accepted = true
                            Theme.toggleMode()
                        }
                    }
                }

                // Power menu (expanded only): opens the power panel
                // (lock / logout / sleep / reboot / shutdown). Actions run
                // through `power_main session …`, which holds the final logic.
                Rectangle {
                    z: 100
                    width: 44
                    height: 26
                    radius: 13
                    color: "transparent"
                    opacity: root.isExpanded ? 1 : 0
                    scale: root.isExpanded ? 1.0 : 0.85
                    transformOrigin: Item.Center

                    Behavior on opacity {
                        NumberAnimation {
                            duration: root.fadeDur
                            easing.type: Easing.OutQuad
                        }
                    }
                    Behavior on scale {
                        NumberAnimation {
                            duration: root.scaleDur
                            easing.type: Easing.OutCubic
                        }
                    }

                    Rectangle {
                        anchors.fill: parent
                        radius: 13
                        color: powerChipHover.containsMouse ? Theme.hover : Theme.bg3

                        Behavior on color {
                            ColorAnimation { duration: 120 }
                        }

                        StatusIcon {
                            anchors.centerIn: parent
                            type: "power"
                            iconSize: 16
                            color: powerChipHover.containsMouse ? Theme.sfg : Theme.fg
                        }
                    }

                    MouseArea {
                        id: powerChipHover
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: function(mouse) {
                            mouse.accepted = true
                            StateController.powerMenu()
                        }
                    }
                }
            }

            // ---- Middle status strip (expanded only) ----
            // Fills the dead gap between the row-1 chip band and the info row
            // with audio + power status: mic, volume, battery time remaining
            // and net up/down speed. (WiFi / Bluetooth now live in the row-1
            // status cluster, right of the tray.) Everything is event-driven
            // or gated on the expanded bar — the 3s net sampler (netTicker)
            // only runs while expanded, so a collapsed bar still costs zero
            // processes.
            Item {
                id: statusStrip
                x: 16
                // Centered in the gap between the row-1 band (ends at y=44)
                // and the info row (starts at y=112): the taller strip stacks
                // the hero clock (dead-center) above the audio/power status.
                y: 44
                width: pill.width - 32
                height: 56
                opacity: root.isExpanded ? 1 : 0
                scale: root.isExpanded ? 1.0 : 0.9
                transformOrigin: Item.Top
                enabled: root.isExpanded

                Behavior on opacity {
                    NumberAnimation {
                        duration: root.fadeDur
                        easing.type: Easing.OutQuad
                    }
                }
                Behavior on scale {
                    NumberAnimation {
                        duration: root.scaleDur
                        easing.type: Easing.OutCubic
                    }
                }

                // Net up/down speed: left-anchored in the status strip with a
                // fixed width, so the live rate labels ("12B" -> "1.2M")
                // never nudge the centered hero clock or the right-side
                // mic / volume / battery-time cluster.
                Item {
                    id: netChip
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    width: 110
                    height: 22

                    Row {
                        spacing: 6

                        StatusIcon {
                            anchors.verticalCenter: parent.verticalCenter
                            type: "arrowUp"
                            iconSize: 13
                            color: Theme.success
                        }

                        Txt {
                            anchors.verticalCenter: parent.verticalCenter
                            text: root.netUpText
                            font.family: Theme.fontName
                            font.pixelSize: 12
                            color: Theme.fg2
                        }

                        StatusIcon {
                            anchors.verticalCenter: parent.verticalCenter
                            type: "arrowDown"
                            iconSize: 13
                            color: Theme.info
                        }

                        Txt {
                            anchors.verticalCenter: parent.verticalCenter
                            text: root.netDownText
                            font.family: Theme.fontName
                            font.pixelSize: 12
                            color: Theme.fg2
                        }
                    }
                }

                Item {
                    id: heroClockBlock
                    // Dead-centered hero clock for the expanded status
                    // strip: hh:mm with the minutes in the accent color and
                    // today's date beneath. Click opens the calendar.
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.verticalCenter: parent.verticalCenter
                    height: parent.height
                    width: 200

                    Column {
                        anchors.centerIn: parent
                        spacing: 0

                        Row {
                            id: heroTimeRow
                            anchors.horizontalCenter: parent.horizontalCenter
                            spacing: 2

                            Txt {
                                text: Qt.formatTime(root.now, "hh")
                                font.family: Theme.fontName
                                font.pixelSize: 34
                                font.weight: Font.Bold
                                font.letterSpacing: 2
                                color: Theme.fg
                            }

                            Txt {
                                text: ":"
                                font.family: Theme.fontName
                                font.pixelSize: 34
                                font.weight: Font.Bold
                                color: Theme.fg3
                            }

                            Txt {
                                text: Qt.formatTime(root.now, "mm")
                                font.family: Theme.fontName
                                font.pixelSize: 34
                                font.weight: Font.Bold
                                font.letterSpacing: 2
                                color: Theme.acc
                            }
                        }
                    }

                    MouseArea {
                        id: heroClockHover
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            mouse.accepted = true
                            StateController.calendar()
                        }
                    }
                }

                Row {
                    id: stripRight
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 14

                    // Mic state: mute glyph + level. Click toggles the mic
                    // (StateController toasts the change via micWatch).
                    Item {
                        id: micChip
                        anchors.verticalCenter: parent.verticalCenter
                        width: micChipRow.implicitWidth
                        height: 22

                        Row {
                            id: micChipRow
                            anchors.fill: parent
                            spacing: 5

                            // Drawn mic icon (shared StatusIcon); red + slash
                            // when muted.
                            StatusIcon {
                                anchors.verticalCenter: parent.verticalCenter
                                type: "mic"
                                muted: VolumeManager.micMuted
                                color: VolumeManager.micMuted ? Theme.danger : Theme.success
                                iconSize: 18
                            }

                            Txt {
                                anchors.verticalCenter: parent.verticalCenter
                                text: Math.round(VolumeManager.micVolume * 100) + "%"
                                font.family: Theme.fontName
                                font.pixelSize: 12
                                color: VolumeManager.micMuted ? Theme.danger : Theme.fg3
                            }
                        }

                        MouseArea {
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: function(mouse) {
                                mouse.accepted = true
                                VolumeManager.toggleMic()
                            }
                        }
                    }

                    // Volume level. Click = audio routing panel (sliders there).
                    Item {
                        id: volChip
                        anchors.verticalCenter: parent.verticalCenter
                        width: volChipRow.implicitWidth
                        height: 22

                        Row {
                            id: volChipRow
                            anchors.fill: parent
                            spacing: 5

                            // Drawn volume icon (shared StatusIcon): waves
                            // track the level, slash when muted (level 0).
                            StatusIcon {
                                anchors.verticalCenter: parent.verticalCenter
                                type: "volume"
                                level: VolumeManager.volumeValue
                                muted: VolumeManager.volumeValue <= 0
                                color: Theme.fg
                                iconSize: 18
                            }

                            Txt {
                                anchors.verticalCenter: parent.verticalCenter
                                text: Math.round(VolumeManager.volumeValue * 100) + "%"
                                font.family: Theme.fontName
                                font.pixelSize: 12
                                color: Theme.fg3
                            }
                        }

                        MouseArea {
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: function(mouse) {
                                mouse.accepted = true
                                StateController.audio()
                            }
                        }
                    }

                    // Battery time remaining ("2h 13m"), hidden when unknown
                    // (AC desktop / full battery / UPower idle).
                    Row {
                        visible: PowerManager.timeRemainingText.length > 0
                        spacing: 5
                        anchors.verticalCenter: parent.verticalCenter

                        StatusIcon {
                            anchors.verticalCenter: parent.verticalCenter
                            type: "clock"
                            iconSize: 13
                            color: Theme.fg3
                        }

                        Txt {
                            anchors.verticalCenter: parent.verticalCenter
                            text: PowerManager.timeRemainingText
                            font.family: Theme.fontName
                            font.pixelSize: 12
                            color: Theme.fg3
                        }
                    }
                }
            }

            // Collapsed strip: full ordered chip row (clock always present).
            // Centered in the island; Settings → Pill owns the order.
            Row {
                id: collapsedItemsRow
                z: 300
                enabled: !root.isExpanded
                // Single gap for every visible chip (clock, EQ, battery…).
                spacing: 10
                x: (root.collapsedWidth - width) / 2
                anchors.verticalCenter: row1Anchor.verticalCenter
                opacity: root.isExpanded ? 0 : 1
                scale: root.isExpanded ? 0.85 : 1.0
                transformOrigin: Item.Center

                Behavior on opacity {
                    NumberAnimation {
                        duration: root.fadeDur
                        easing.type: Easing.OutQuad
                    }
                }
                Behavior on scale {
                    NumberAnimation {
                        duration: root.scaleDur
                        easing.type: Easing.OutCubic
                    }
                }
                Behavior on x {
                    NumberAnimation {
                        duration: 240
                        easing.type: Easing.OutCubic
                    }
                }

                Repeater {
                    model: root.collapsedItemOrder

                    delegate: Loader {
                        id: chipLoader
                        required property var modelData
                        required property int index
                        anchors.verticalCenter: parent.verticalCenter
                        readonly property string chipKey: String(modelData || "")
                        active: chipKey.length > 0
                        // Keep slot in layout only when shown; width is content only.
                        visible: status === Loader.Ready && item && item.showChip
                        width: (status === Loader.Ready && item && item.showChip) ? item.chipWidth : 0
                        height: 28

                        sourceComponent: {
                            switch (chipLoader.chipKey) {
                            case "clock": return root.collapsedClockComp
                            case "musicBarAnim": return root.collapsedEqComp
                            case "collapsedBattery": return root.collapsedBattComp
                            case "collapsedWatts": return root.collapsedWattsComp
                            case "collapsedWeather": return root.collapsedWeatherComp
                            case "collapsedNotification": return root.collapsedNotifComp
                            case "collapsedWorkspace": return root.collapsedWsComp
                            case "collapsedSettings": return root.collapsedSettingsComp
                            case "collapsedPower": return root.collapsedPowerComp
                            default: return null
                            }
                        }
                    }
                }
            }

            // ---- Row 2: info strip (expanded only) ----
            // Now-playing on the left (album art, title, artist, controls),
            // the vertical hour/minute clock dead-center, the compact date
            // strip (today highlighted) and weather on the right. Fades/
            // scales in with the rest of the chips; only interactive while
            // expanded.
            Item {
                id: infoRow
                x: 16
                // Bottom-pinned info strip; the vertical clock (info row)
                // replaces the old dead-center pill clock on expand.
                y: root.expandedHeight - 136 - 8
                width: pill.width - 32
                height: 136
                opacity: root.isExpanded ? 1 : 0
                scale: root.isExpanded ? 1.0 : 0.9
                transformOrigin: Item.Top
                enabled: root.isExpanded

                Behavior on opacity {
                    NumberAnimation {
                        duration: root.fadeDur
                        easing.type: Easing.OutQuad
                    }
                }
                Behavior on scale {
                    NumberAnimation {
                        duration: root.scaleDur
                        easing.type: Easing.OutCubic
                    }
                }

                Item {
                    id: mprisBlock
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    // Slimmed from 450 so the dead-center vertical clock
                    // and the calendar strip fit in the same info row.
                    width: 330
                    height: parent.height

                    // Album art: rounded corners via OpacityMask — Rectangle.clip
                    // only clips to the bounding box, which leaves square image
                    // corners. The mask matches the card's radius (MorphContainer
                    // uses the same pattern). The glyph fallback stays visible
                    // until the art actually loads.
                    Rectangle {
                        id: artBox
                        width: 110
                        height: 110
                        radius: 26
                        anchors.left: parent.left
                        anchors.verticalCenter: parent.verticalCenter
                        color: Theme.bg3
                        clip: true
                        border.width: 2
                        border.color: Theme.bg3

                        Image {
                            id: artImage
                            anchors.fill: parent
                            visible: MprisManager.artUrl.length > 0 && status === Image.Ready
                            source: MprisManager.artUrl
                            fillMode: Image.PreserveAspectCrop
                            antialiasing: true
                            layer.enabled: true
                            layer.samples: 2
                            layer.effect: OpacityMask {
                                maskSource: artMask
                            }
                        }

                        // Invisible mask that gives the art its rounded corners.
                        Rectangle {
                            id: artMask
                            anchors.fill: parent
                            radius: 26
                            color: "white"
                            visible: false
                        }

                        Txt {
                            anchors.centerIn: parent
                            visible: MprisManager.artUrl.length === 0
                            text: MprisManager.status === "Playing" ? "\uf04c" : "\uf04b"
                            font.family: Theme.iconFont
                            font.pixelSize: 40
                            color: Theme.fg
                        }
                    }

                    // Title / artist stacked, with the transport controls
                    // BELOW them (matches the reference layout).
                    Column {
                        anchors.left: artBox.right
                        anchors.leftMargin: 14
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 6

                        Txt {
                            width: parent.width
                            elide: Text.ElideRight
                            text: MprisManager.title.length > 0 ? MprisManager.title : "No media playing"
                            color: Theme.fg
                            font.family: Theme.fontName
                            font.pixelSize: 16
                            font.bold: true
                        }

                        Txt {
                            width: parent.width
                            elide: Text.ElideRight
                            text: MprisManager.artist.length > 0 ? MprisManager.artist : "Open a player"
                            color: Theme.fg3
                            font.family: Theme.fontName
                            font.pixelSize: 13
                        }

                        Row {
                            id: controlsRow
                                spacing: 10

                            Rectangle {
                                width: 24
                                height: 24
                                radius: 12
                                color: controlsPrev.containsMouse ? Theme.hover : "transparent"

                                StatusIcon {
                                    anchors.centerIn: parent
                                    type: "prev"
                                    iconSize: 14
                                    color: Theme.fg
                                }

                                MouseArea {
                                    id: controlsPrev
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: function(mouse) {
                                        mouse.accepted = true
                                        MprisManager.previous()
                                    }
                                }
                            }

                            Rectangle {
                                width: 30
                                height: 30
                                radius: 15
                                color: Theme.acc

                                StatusIcon {
                                    anchors.centerIn: parent
                                    type: MprisManager.status === "Playing" ? "pause" : "play"
                                    iconSize: 16
                                    color: Theme.sfg
                                }

                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: function(mouse) {
                                        mouse.accepted = true
                                        MprisManager.playPause()
                                    }
                                }
                            }

                            Rectangle {
                                width: 24
                                height: 24
                                radius: 12
                                color: controlsNext.containsMouse ? Theme.hover : "transparent"

                                StatusIcon {
                                    anchors.centerIn: parent
                                    type: "next"
                                    iconSize: 14
                                    color: Theme.fg
                                }

                                MouseArea {
                                    id: controlsNext
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: function(mouse) {
                                        mouse.accepted = true
                                        MprisManager.next()
                                    }
                                }
                            }
                        }

                            // ---- Media progress (stacks inside the
                            // title/artist Column; no verticalCenter
                            // anchor here or the Column breaks) ----
                            Item {
                                id: mediaProgressRow
                                width: parent.width
                                height: 12

                                // Expanded EQ always while Playing (no settings gate).
                                Row {
                                    id: expandedEq
                                    anchors.left: parent.left
                                    anchors.verticalCenter: parent.verticalCenter
                                    spacing: 2.5
                                    height: 10
                                    width: root.expandedMusicEq ? implicitWidth : 0
                                    opacity: root.expandedMusicEq ? 1 : 0
                                    clip: true
                                    visible: width > 0.5

                                    Behavior on width {
                                        NumberAnimation {
                                            duration: 120
                                            easing.type: Easing.OutQuad
                                        }
                                    }
                                    Behavior on opacity {
                                        NumberAnimation {
                                            duration: 120
                                            easing.type: Easing.OutQuad
                                        }
                                    }

                                    Repeater {
                                        model: 4
                                        delegate: Rectangle {
                                            required property int index
                                            width: 2.5
                                            radius: 1
                                            anchors.bottom: parent ? parent.bottom : undefined
                                            color: Theme.acc
                                            height: 3

                                            SequentialAnimation on height {
                                                loops: Animation.Infinite
                                                running: root.expandedMusicEq
                                                PauseAnimation { duration: index * 70 }
                                                NumberAnimation {
                                                    from: 3
                                                    to: 10
                                                    duration: 260 + index * 35
                                                    easing.type: Easing.InOutSine
                                                }
                                                NumberAnimation {
                                                    from: 10
                                                    to: 3
                                                    duration: 260 + index * 35
                                                    easing.type: Easing.InOutSine
                                                }
                                            }
                                        }
                                    }
                                }

                                Rectangle {
                                    height: 2
                                    radius: 1
                                    color: Theme.bg3
                                    anchors.left: expandedEq.right
                                    anchors.leftMargin: root.expandedMusicEq ? 8 : 0
                                    anchors.right: mprisTime.left
                                    anchors.rightMargin: 8
                                    anchors.verticalCenter: parent.verticalCenter

                                    Rectangle {
                                        id: mediaProgress
                                        width: parent.width * (MprisManager.mediaPosition / Math.max(1, MprisManager.mediaLength))
                                        height: parent.height
                                        radius: 1
                                        color: Theme.acc
                                    }
                                }

                                Txt {
                                    id: mprisTime
                                    anchors.right: parent.right
                                    anchors.verticalCenter: parent.verticalCenter
                                    font.family: Theme.fontName
                                    font.pixelSize: 10
                                    color: Theme.fg3
                                    text: root.fmtTime(MprisManager.mediaPosition)
                                }
                            }

                    }
                }

                    // ---- Date strip (between clock and weather) ----
                    Item {
                        id: dateBlock
                        anchors.left: mprisBlock.right
                        anchors.right: weatherBlock.left
                        anchors.verticalCenter: parent.verticalCenter
                        height: parent.height

                        Column {
                            anchors.centerIn: parent
                            spacing: 5

                            // Month + year above the week strip (e.g. "Aug, 2026").
                            Txt {
                                anchors.horizontalCenter: parent.horizontalCenter
                                text: Qt.formatDate(root.now, "MMM, yyyy")
                                color: Theme.acc
                                font.family: Theme.fontName
                                font.pixelSize: 12
                                font.bold: true
                            }

                            Row {
                                spacing: 5
                                anchors.horizontalCenter: parent.horizontalCenter

                                    Repeater {
                                        model: root.dayCells()
                                        delegate: Column {
                                            required property var modelData
                                            spacing: 1
                                            width: 26

                                            Txt {
                                                anchors.horizontalCenter: parent.horizontalCenter
                                                text: modelData.wd
                                                color: modelData.isToday ? Theme.acc : Theme.fg3
                                                font.family: Theme.fontName
                                                font.pixelSize: 9
                                                font.bold: modelData.isToday
                                            }

                                            Rectangle {
                                                width: 26
                                                height: 26
                                                radius: 13
                                                anchors.horizontalCenter: parent.horizontalCenter
                                                color: modelData.isToday ? Theme.acc : "transparent"

                                                Txt {
                                                    anchors.centerIn: parent
                                                    text: modelData.day
                                                    color: modelData.isToday ? Theme.sfg : Theme.fg
                                                    font.family: Theme.fontName
                                                    font.pixelSize: 11
                                                    font.bold: modelData.isToday
                                                }
                                            }
                                        }
                                    }
                            }

                            Txt {
                                anchors.horizontalCenter: parent.horizontalCenter
                                width: 210
                                horizontalAlignment: Text.AlignHCenter
                                elide: Text.ElideRight
                                // Only today's special occasion is shown —
                                // nothing renders when there are no events.
                                visible: CalendarManager.eventsOn(root.now.getMonth(), root.now.getDate()).length > 0
                                text: "\uf133 " + CalendarManager.eventsOn(root.now.getMonth(), root.now.getDate()).join(" · ")
                                color: Theme.fg3
                                font.family: Theme.fontName
                                font.pixelSize: 10
                            }
                        }
                    }

                    // ---- Weather widget (right) ----
                    Item {
                        id: weatherBlock
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        width: 96
                        height: parent.height

                        Column {
                            anchors.centerIn: parent
                            spacing: 3

                            Rectangle {
                                id: weatherBtn
                                width: 56
                                height: 56
                                radius: 28
                                anchors.horizontalCenter: parent.horizontalCenter
                                color: weatherHover.containsMouse ? Theme.hover : Theme.bg2
                                border.width: 1
                                border.color: Theme.bg3

                                Behavior on color {
                                    ColorAnimation { duration: 120 }
                                }

                                Txt {
                                    anchors.centerIn: parent
                                    text: WeatherManager.glyph.length > 0 ? WeatherManager.glyph : "\uf0c2"
                                    font.family: Theme.iconFont
                                    font.pixelSize: 26
                                    color: Theme.acc
                                }

                                MouseArea {
                                    id: weatherHover
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: function(mouse) {
                                        mouse.accepted = true
                                        WeatherManager.refresh()
                                        StateController.weather()
                                    }
                                }
                            }

                            Txt {
                                anchors.horizontalCenter: parent.horizontalCenter
                                text: WeatherManager.ready ? WeatherManager.temperature + "\u00b0" : "--\u00b0"
                                color: Theme.fg
                                font.family: Theme.fontName
                                font.pixelSize: 13
                                font.bold: true
                            }

                            Txt {
                                anchors.horizontalCenter: parent.horizontalCenter
                                width: 88
                                elide: Text.ElideRight
                                horizontalAlignment: Text.AlignHCenter
                                text: WeatherManager.ready ? WeatherManager.condition : "Weather"
                                color: Theme.fg3
                                font.family: Theme.fontName
                                font.pixelSize: 9
                            }

                            // Sunrise / sunset from the same Open-Meteo fetch
                            // (hidden until the first successful forecast).
                            Row {
                                visible: WeatherManager.sunriseText.length > 0
                                anchors.horizontalCenter: parent.horizontalCenter
                                spacing: 5

                                Txt {
                                    text: "\uf185"
                                    font.family: Theme.fontName
                                    font.pixelSize: 8
                                    color: Theme.fg3
                                }

                                Txt {
                                    text: WeatherManager.sunriseText
                                    font.family: Theme.fontName
                                    font.pixelSize: 9
                                    color: Theme.fg3
                                }

                                Txt {
                                    text: "\uf186"
                                    font.family: Theme.fontName
                                    font.pixelSize: 8
                                    color: Theme.fg3
                                }

                                Txt {
                                    text: WeatherManager.sunsetText
                                    font.family: Theme.fontName
                                    font.pixelSize: 9
                                    color: Theme.fg3
                                }
                            }
                        }
                    }
            }
        }

        // Hidden white copy of the silhouette used to clip the pill's
        // content (chips + info row) to the island shape so
        // nothing bleeds past the tapered walls — same pattern as
        // MorphContainer's maskShape for panels. Kept as a SIBLING of the
        // pill (not a child) so the OpacityMask samples a stable texture
        // instead of the pill's own layer output.
        Shape {
            id: pillMask
            anchors.centerIn: parent
            width: pill.width
            height: pill.height
            layer.enabled: true
            layer.samples: 2
            antialiasing: true
            visible: false

            // Mirrors the pill's own topR/botR (capsule when floating,
            // flat bottom + small top radius when stuck to the top). Same
            // Behaviors so mask and shape morph together.
            property real topR: root.stuckTop ? 8 : height / 2
            property real botR: root.stuckTop ? 0 : height / 2

            Behavior on topR {
                NumberAnimation { duration: 150; easing.type: Easing.OutCubic }
            }
            Behavior on botR {
                NumberAnimation { duration: 150; easing.type: Easing.OutCubic }
            }

            ShapePath {
                strokeWidth: 0
                fillColor: "white"
                strokeColor: "transparent"

                startX: pillMask.topR
                startY: 0

                PathLine {
                    x: pillMask.width - pillMask.topR
                    y: 0
                }

                PathQuad {
                    x: pillMask.width
                    y: pillMask.topR
                    controlX: pillMask.width
                    controlY: 0
                }

                PathLine {
                    x: pillMask.width
                    y: pillMask.height - pillMask.botR
                }

                PathQuad {
                    x: pillMask.width - pillMask.botR
                    y: pillMask.height
                    controlX: pillMask.width
                    controlY: pillMask.height
                }

                PathLine {
                    x: pillMask.botR
                    y: pillMask.height
                }

                PathQuad {
                    x: 0
                    y: pillMask.height - pillMask.botR
                    controlX: 0
                    controlY: pillMask.height
                }

                PathLine {
                    x: 0
                    y: pillMask.topR
                }

                PathQuad {
                    x: pillMask.topR
                    y: 0
                    controlX: 0
                    controlY: 0
                }
            }
        }
    }

    // Hover area for the pill - exactly matches pill dimensions
    // (pill is nested inside hoverArea, so anchor to the wrapper).
    // Right-click anywhere on the island opens Settings.
    MouseArea {
        id: pillHover
        anchors.fill: hoverArea
        z: 1000
        hoverEnabled: true
        acceptedButtons: Qt.RightButton
        cursorShape: Qt.PointingHandCursor
        onEntered: {
            if (root.hoverCollapse) {
                root.hovered = true
            }
        }
        onExited: {
            if (root.hoverCollapse) {
                root.hovered = false
            }
        }
        onClicked: function(mouse) {
            // Collapsed only — expanded dashboard must not trap into settings.
            if (mouse.button === Qt.RightButton && !root.isExpanded) {
                mouse.accepted = true
                StateController.settings()
            }
        }
    }
}