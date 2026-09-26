import QtQuick
import QtQuick.Shapes
import Qt5Compat.GraphicalEffects
import "."
import "../services"

Item {
    id: root

    property string mode: "pill"
    property Component pillComponent
    property var components: ({})
    // Stuck-to-top mode (floating off) keeps the same smooth rounded corners
    // as floating — the island just sits flush at the screen top (MorphSurface
    // animates the top gap to 0). Mirrors Pill.stuckTop.
    property bool stuckTop: SettingsManager.config.shell.floating === false
    // Fullscreen panel (settings): square corners, island covers the whole
    // screen — the rounded-cap silhouette is dropped entirely.
    property bool fullscreen: false

    // The island keeps its rounded pill corners in every mode; sticking to
    // the top only removes the top gap (MorphSurface animates the margin).

    property real targetWidth: 0
    property real targetHeight: 0
    // True once the box animation has landed on its target. While false,
    // nextWidth/nextHeight keep the *previous* size so the window can size
    // itself to max(current, target, next) — the window resizes at most
    // twice per morph instead of every frame (each frame = one layer-shell
    // configure round-trip to the compositor, the jank amplifier when a
    // toast window is also up).
    readonly property bool settled: Math.abs(root.width - root.targetWidth) < 0.5 && Math.abs(root.height - root.targetHeight) < 0.5

    onSettledChanged: {
        if (root.settled) {
            root.nextWidth = root.width
            root.nextHeight = root.height
        }
    }

    property real nextWidth: 0
    property real nextHeight: 0
    property bool collapsingToPill: false
    property real contentOpacity: 1
    property bool poppedIn: false

    width: targetWidth
    height: targetHeight

    // No springs anywhere: a spring overshoots its target and oscillates,
    // and since the content is anchored inside the growing box, box and
    // content visibly bounce together. Expand and collapse both use a plain
    // ease-out so nothing oscillates (Apple dynamic-island style). Collapse
    // is slightly faster than expand.
    Behavior on width {
        NumberAnimation {
            duration: root.collapsingToPill ? 200 : 240
            easing.type: Easing.OutCubic
        }
    }

    Behavior on height {
        NumberAnimation {
            duration: root.collapsingToPill ? 200 : 240
            easing.type: Easing.OutCubic
        }
    }

    // Master/universal background for all modes
    Shape {
        id: surface
        anchors.fill: parent
        layer.enabled: true
        layer.samples: 2
        antialiasing: true

        // Silhouette: floating = the pill is a true capsule (radius = half
        // the height); panels are tastefully rounded (capped so a tall panel
        // doesn't balloon). Stuck-to-top (floating off) = a flat bar flush
        // at the screen top: small top radius, straight bottom edge. The
        // Behaviors smooth the radius jump when a panel opens from an
        // expanded capsule pill.
        property bool isPill: root.mode === "pill"
        property real topR: root.fullscreen ? 0 : (root.stuckTop ? 8 : (root.isPill ? root.height / 2 : Math.min(28, root.height / 2)))
        property real botR: root.fullscreen ? 0 : (root.stuckTop ? 0 : (root.isPill ? root.height / 2 : Math.min(28, root.height / 2)))

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

            startX: surface.topR
            startY: 0

            // Top edge
            PathLine {
                x: surface.width - surface.topR
                y: 0
            }

            // Top-right corner — rounded
            PathQuad {
                x: surface.width
                y: surface.topR
                controlX: surface.width
                controlY: 0
            }

            // Right side
            PathLine {
                x: surface.width
                y: surface.height - surface.botR
            }

            // Bottom-right — deep U curve
            PathQuad {
                x: surface.width - surface.botR
                y: surface.height
                controlX: surface.width
                controlY: surface.height
            }

            // Bottom edge
            PathLine {
                x: surface.botR
                y: surface.height
            }

            // Bottom-left — deep U curve
            PathQuad {
                x: 0
                y: surface.height - surface.botR
                controlX: 0
                controlY: surface.height
            }

            // Left side
            PathLine {
                x: 0
                y: surface.topR
            }

            // Top-left corner — rounded
            PathQuad {
                x: surface.topR
                y: 0
                controlX: 0
                controlY: 0
            }
        }
    }

    Shape {
        id: maskShape
        anchors.fill: parent
        layer.enabled: true
        layer.samples: 2
        antialiasing: true
        visible: false

        // Mirror of `surface` above (same silhouette rules); this is the
        // white mask that clips panel content to the rounded box. Same
        // Behaviors so mask and shape morph together.
        property bool isPill: root.mode === "pill"
        property real topR: root.fullscreen ? 0 : (root.stuckTop ? 8 : (root.isPill ? root.height / 2 : Math.min(28, root.height / 2)))
        property real botR: root.fullscreen ? 0 : (root.stuckTop ? 0 : (root.isPill ? root.height / 2 : Math.min(28, root.height / 2)))

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

            startX: maskShape.topR
            startY: 0

            // Top edge
            PathLine {
                x: maskShape.width - maskShape.topR
                y: 0
            }

            // Top-right corner — rounded
            PathQuad {
                x: maskShape.width
                y: maskShape.topR
                controlX: maskShape.width
                controlY: 0
            }

            // Right side
            PathLine {
                x: maskShape.width
                y: maskShape.height - maskShape.botR
            }

            // Bottom-right — deep U curve
            PathQuad {
                x: maskShape.width - maskShape.botR
                y: maskShape.height
                controlX: maskShape.width
                controlY: maskShape.height
            }

            // Bottom edge
            PathLine {
                x: maskShape.botR
                y: maskShape.height
            }

            // Bottom-left — deep U curve
            PathQuad {
                x: 0
                y: maskShape.height - maskShape.botR
                controlX: 0
                controlY: maskShape.height
            }

            // Left side
            PathLine {
                x: 0
                y: maskShape.topR
            }

            // Top-left corner — rounded
            PathQuad {
                x: maskShape.topR
                y: 0
                controlX: 0
                controlY: 0
            }
        }
    }

    Loader {
        id: loader
        anchors.centerIn: parent
        opacity: root.contentOpacity
        // Content is scaled to always fit the box (see fitScale), so it
        // expands together with the island instead of being rendered at
        // full size and revealed/cropped by the growing box.
        scale: root.fitScale()

        sourceComponent:
            root.mode === "pill"
            ? root.pillComponent
            : root.components[root.mode]

        // The pill is only masked while it's a panel (content needs clipping
        // to the rounded box). In pill mode the bar renders without a layer:
        // text inside a re-rasterized layer shimmers at fractional offsets,
        // which read as clock flicker while the pill morphs.
        layer.enabled: root.mode !== "pill"
        layer.effect: OpacityMask {
            maskSource: maskShape
        }

        Behavior on opacity {
            id: contentOpacityBehavior
            NumberAnimation {
                duration: root.collapsingToPill ? 110 : 140
                easing.type: Easing.OutCubic
            }
        }

        onLoaded: {
            if (item)
                Qt.callLater(root.updateSize)

            // Fade the content in as the box expands around it. Scale is
            // handled by fitScale (bound to the box size), so there is no
            // separate pop to cause a swelling/pulsing look.
            if (item && root.mode !== "pill" && root.mode !== "dummy" && !root.poppedIn) {
                root.poppedIn = true
                contentOpacityBehavior.enabled = false
                root.contentOpacity = 0
                contentOpacityBehavior.enabled = true
                fadeInTimer.start()
            }
        }
    }

    // Release the fade one frame later so the opacity Behavior animates
    // from 0 to 1 instead of from the previous panel's state.
    Timer {
        id: fadeInTimer
        interval: 24
        repeat: false
        onTriggered: root.contentOpacity = 1
    }

    // Scale factor that keeps the loaded content inside the box while it
    // grows: 1 when the box is at least as large as the content, less
    // while the island is still expanding. This makes the content expand
    // together with the box (Apple dynamic-island style) instead of being
    // clipped by it.
    function fitScale() {
        // The pill is the bar itself and grows in lockstep with the box, so
        // it is never scaled (scaling it would double-morph and make the
        // clock text shimmer). Only panels are scaled to fit while the box
        // expands around them.
        if (root.mode === "pill")
            return 1
        var it = loader.item
        if (!it)
            return 1
        var w = it.implicitWidth > 0 ? it.implicitWidth : it.width
        var h = it.implicitHeight > 0 ? it.implicitHeight : it.height
        if (w <= 0 || h <= 0)
            return 1
        return Math.min(1, root.width / w, root.height / h)
    }

    function updateSize() {
        if (!loader.item)
            return

        var w = loader.item.implicitWidth
        var h = loader.item.implicitHeight

        if (w <= 0)
            w = loader.item.width

        if (h <= 0)
            h = loader.item.height

        if (w <= 0 || h <= 0)
            return

        targetWidth = w
        targetHeight = h

        if (mode !== "dummy" && root.settled) {
            nextWidth = w
            nextHeight = h
        }
    }

    Connections {
        target: loader.item || null

        function onWidthChanged() {
            root.updateSize()
        }

        function onHeightChanged() {
            root.updateSize()
        }

        function onImplicitWidthChanged() {
            root.updateSize()
        }

        function onImplicitHeightChanged() {
            root.updateSize()
        }
    }

    onModeChanged: {
        root.poppedIn = false
        root.collapsingToPill = root.mode === "dummy" && StateController.osdMode === "pill"

        // Fade the content out while collapsing to the pill; fade it back
        // in once the pill (or a new panel) is loaded. Scale is entirely
        // driven by fitScale, so no spring/pop values here.
        root.contentOpacity = root.collapsingToPill ? 0 : 1

        Qt.callLater(updateSize)
    }
}