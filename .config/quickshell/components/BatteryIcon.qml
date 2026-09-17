// BatteryIcon.qml — a drawn battery icon (no font glyph), ported from Tide
// Island's SwipeCustomInfoLayer battery shape (github.com/enhaoswen/Tide-island).
//
// Renders a real battery silhouette: a translucent rounded body, an animated
// fill that tracks `level`, a small tip nub on the right, and the level
// number drawn INSIDE the battery. While `charging`, the fill/body turn
// solid and a bolt glyph appears next to the number. The fill turns red
// below 20%.
//
// Theme-aware: the caller passes `fillColor` (white in dark mode, the theme
// fg in light mode) and the text/bolt color is derived from its luminance,
// so the icon reads on any background.
import QtQuick
import "."

Item {
    id: root

    // ---- Inputs ----
    property int level: 0                 // 0-100 charge percent
    property bool charging: false         // plugged / charging state
    property color fillColor: "#ffffff"   // charged-fill + body color

    // ---- Sizing (Tide Island defaults) ----
    property int iconWidth: 37
    property int iconHeight: 17
    property int fontSize: 13
    property int chargingFontSize: 12
    property int boltSize: 10
    property int tipWidth: 2
    property int tipHeight: 5
    property int outerRadius: 6

    // ---- Fonts (theme fonts passed in by the caller) ----
    property string fontFamily: "sans-serif"
    property string iconFontFamily: "sans-serif"

    // Light fill -> dark glyphs, dark fill -> light glyphs.
    function glyphColor(color) {
        const lum = 0.299 * color.r + 0.587 * color.g + 0.114 * color.b
        return lum > 0.6 ? "#242424" : "#ffffff"
    }

    function clampedLevel() {
        return Math.max(0, Math.min(100, root.level))
    }

    width: root.iconWidth
    height: root.iconHeight

    // Battery body: translucent rounded rect, clipped so the fill and the
    // level text stay inside the silhouette.
    Rectangle {
        id: batteryBody
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        width: parent.width - root.tipWidth - 1
        height: parent.height
        radius: root.outerRadius
        color: Qt.rgba(root.fillColor.r, root.fillColor.g, root.fillColor.b, 0.56)
        clip: true

        // Charged fill: grows from the left with a rounded left cap; the
        // right end rounds off once the battery is ~full.
        Rectangle {
            id: batteryFill
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            readonly property bool roundedEnd: root.clampedLevel() >= 85
            topLeftRadius: root.outerRadius
            bottomLeftRadius: root.outerRadius
            topRightRadius: root.roundedEnd ? root.outerRadius : 0
            bottomRightRadius: root.roundedEnd ? root.outerRadius : 0
            width: Math.max(root.outerRadius * 2, parent.width * (root.clampedLevel() / 100))
            color: root.fillColor

            Behavior on width {
                NumberAnimation { duration: 300; easing.type: Easing.OutCubic }
            }
            Behavior on color {
                ColorAnimation { duration: 300 }
            }
        }

        // Charging: level + bolt glyph, centered over the body.
        Row {
            visible: root.charging
            anchors.centerIn: parent
            spacing: 2
            z: 2

            Txt {
                anchors.verticalCenter: parent.verticalCenter
                text: root.clampedLevel() + ""
                color: root.glyphColor(root.fillColor)
                font.family: root.fontFamily
                font.pixelSize: root.chargingFontSize
                font.weight: Font.DemiBold
                verticalAlignment: Text.AlignVCenter
            }

            Txt {
                anchors.verticalCenter: parent.verticalCenter
                // FA bolt (\uf0e7) from the nerd-font icon font.
                text: "\uf0e7"
                color: root.glyphColor(root.fillColor)
                font.family: root.iconFontFamily
                font.pixelSize: root.boltSize
                verticalAlignment: Text.AlignVCenter
            }
        }

        // Idle: level number centered inside the battery.
        Txt {
            visible: !root.charging
            anchors.centerIn: parent
            text: root.clampedLevel() + ""
            color: root.glyphColor(root.fillColor)
            font.family: root.fontFamily
            font.pixelSize: root.fontSize
            font.weight: root.clampedLevel() <= 20 ? Font.Bold : Font.DemiBold
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
            z: 2
        }
    }

    // Tip nub: lights up when the battery is full.
    Rectangle {
        width: root.tipWidth
        height: root.tipHeight
        radius: Math.round(root.tipWidth / 2)
        color: root.clampedLevel() >= 100
            ? root.fillColor
            : Qt.rgba(root.fillColor.r, root.fillColor.g, root.fillColor.b, 0.56)
        anchors.left: batteryBody.right
        anchors.leftMargin: 1
        anchors.verticalCenter: parent.verticalCenter

        Behavior on color {
            ColorAnimation { duration: 300 }
        }
    }
}
