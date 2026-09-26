// StatusIcon.qml — Nerd Font glyphs via Theme.iconFont (FA set).
// API: type, level, active, muted, charging, color, iconSize
import QtQuick
import "."
import "../services"

Item {
    id: root

    property string type: "wifi"
    property real level: 1
    property bool active: true
    property bool muted: false
    property bool charging: false
    property color color: "#ffffff"
    property int iconSize: 22
    property string batteryFontFamily: Theme.fontName
    property string batteryIconFontFamily: Theme.iconFont

    width: root.iconSize
    height: root.iconSize

    function batteryGlyph() {
        const p = Math.max(0, Math.min(100, Math.round(root.level * 100)))
        if (root.charging)
            return "\uf0e7"
        if (p <= 10)
            return "\uf244"
        if (p <= 30)
            return "\uf243"
        if (p <= 55)
            return "\uf242"
        if (p <= 80)
            return "\uf241"
        return "\uf240"
    }

    function glyphText() {
        switch (root.type) {
        case "wifi":
            return "\uf1eb"
        case "bluetooth":
            return "\uf293"
        case "volume":
            if (root.muted || root.level <= 0)
                return "\uf026"
            if (root.level < 0.34)
                return "\uf027"
            return "\uf028"
        case "brightness":
        case "sun":
            return "\uf185"
        case "moon":
            return "\uf186"
        case "mic":
            return root.muted ? "\uf131" : "\uf130"
        case "gear":
            return "\uf013"
        case "sliders":
            // FA sliders-h — same family as Settings "Appearance" paint icon
            return "\uf1de"
        case "grid":
            return "\uf00a"
        case "bell":
            return root.muted ? "\uf1f6" : "\uf0f3"
        case "power":
            return "\uf011"
        case "clock":
            return "\uf017"
        case "arrowUp":
            return "\uf062"
        case "arrowDown":
            return "\uf063"
        case "chevronRight":
            return "\uf054"
        case "chevronLeft":
            return "\uf053"
        case "bolt":
            return "\uf0e7"
        case "monitor":
            return "\uf108"
        case "play":
            return "\uf04b"
        case "pause":
            return "\uf04c"
        case "prev":
            return "\uf048"
        case "next":
            return "\uf051"
        case "check":
            return "\uf00c"
        case "bulb":
            return "\uf0eb"
        case "coffee":
            return "\uf0f4"
        case "music":
            return "\uf001"
        case "refresh":
            return "\uf021"
        case "lock":
            return "\uf023"
        case "lockOpen":
            return "\uf09c"
        case "battery":
            return batteryGlyph()
        case "adjust":
        case "warmth":
            return "\uf042"
        case "palette":
        case "vibrance":
            return "\uf53f"
        default:
            return "\uf128"
        }
    }

    Text {
        anchors.centerIn: parent
        text: root.glyphText()
        font.family: Theme.iconFont
        font.pixelSize: Math.max(9, Math.round(root.iconSize * 0.88))
        color: root.color
        opacity: (root.type === "wifi" || root.type === "bluetooth") && !root.active ? 0.4 : 1
        renderType: Text.NativeRendering
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
    }
}
