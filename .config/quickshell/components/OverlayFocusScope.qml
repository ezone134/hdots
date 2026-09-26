import QtQuick

// Standard focus-on-open behaviour for overlays: grab focus shortly after
// the panel becomes visible (and once on completion for the very first
// open). Drop-in for the repeated `Timer { interval: 150 }` block.
Item {
    id: root

    property int delay: 150
    property Item focusTarget: null

    Component.onCompleted: grab()
    onVisibleChanged: {
        if (visible)
            grab()
    }

    function grab() {
        focusTimer.restart()
    }

    Timer {
        id: focusTimer
        interval: root.delay
        repeat: false
        onTriggered: (root.focusTarget || root).forceActiveFocus()
    }
}
