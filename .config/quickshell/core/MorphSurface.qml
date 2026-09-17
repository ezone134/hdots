import QtQuick
import "."
import "../services"

Item {
    id: root

    property Component pillComponent
    property string mode: "pill"
    property string requestedMode: StateController.osdMode
    property bool stuckTop: SettingsManager.config.shell.floating === false
    // Fullscreen panel (settings): the surface fills the whole window and
    // the +25px transparent margin / top gap are dropped so the island
    // covers the screen edge to edge.
    property bool fullscreen: false

    // The surface always shrink-wraps the container (never full-screen).
    // Stuck-to-top mode only slides the island up to the top edge: the
    // container's top gap animates to 0 while the corners stay rounded
    // (curved top edges, Dynamic-Lake style). The +25px below the bar is
    // transparent, so the window never resizes during the stick animation.
    // In fullscreen the container itself grows to the screen size, so the
    // implicit dims already cover the whole display — the +25 margin is
    // dropped so the window is exactly the screen.
    width: implicitWidth
    // Hold the larger of current/previous/target size so the window only
    // resizes at the morph start (expand) or end (collapse) instead of every
    // animation frame. The transparent margins mean the grow/shrink box is
    // rendered identically; the box is top-anchored so nothing shifts.
    implicitWidth: Math.max(container.width, container.targetWidth, container.nextWidth)
    implicitHeight: Math.max(container.height, container.targetHeight, container.nextHeight)
        + (root.fullscreen ? 0 : 25)

    Component {
        id: dummyComp
        MorphPlaceholder {
            implicitWidth: container.nextWidth
            implicitHeight: container.nextHeight
        }
    }

    MorphContainer {
        id: container
        anchors.top: parent.top
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.topMargin: root.fullscreen ? 0 : (root.stuckTop ? 0 : 25)
        // Smooth rise when the floating toggle turns off and fall when it
        // turns back on — no snap up/down.
        Behavior on anchors.topMargin {
            NumberAnimation { duration: 250; easing.type: Easing.OutCubic }
        }
        mode: root.mode
        pillComponent: root.pillComponent
        fullscreen: root.fullscreen
        // "dummy" stays local (it needs container.nextWidth/Height for the
        // morph); every real panel/OSD lives in PanelRegistry.componentMap.
        components: Object.assign({ "dummy": dummyComp }, PanelRegistry.componentMap)
    }

    Timer {
        id: switchTimer
        interval: 50
        repeat: false
        onTriggered: root.mode = root.requestedMode
    }

    onRequestedModeChanged: {
        if (requestedMode === mode)
            return
        container.nextWidth = container.width
        container.nextHeight = container.height
        mode = "dummy"
        switchTimer.restart()
    }
}