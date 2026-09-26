import QtQuick

// Selection state shared by list overlays: an index clamped/moved over a
// count with wrap-around. Panels bind `count` to their model length and
// call move() from their Keys handlers, keeping the clamp math in one place.
QtObject {
    id: root

    property int count: 0
    property int index: 0

    function clamp() {
        if (count === 0)
            index = 0
        else
            index = Math.max(0, Math.min(index, count - 1))
    }

    function move(delta) {
        if (count === 0)
            return
        index = (index + delta + count) % count
    }

    function reset() {
        index = 0
    }
}
