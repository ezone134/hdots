import QtQuick
import "."
import "../../components"
import "../../services"

// Small transient notification: the bar morphs into this for workspace-switch
// and AC plug/unplug events. Content is driven by the parent mode component
// (see MorphSurface), the background comes from MorphContainer.
Item {
    id: root

    property string icon: "\uf108"
    // When iconType is set, the drawn StatusIcon is used instead of the
    // glyph `icon` (Apple-style: monochrome vector, level/state aware).
    property string iconType: ""
    property real iconLevel: 0
    property bool iconActive: true
    property bool iconMuted: false
    property bool iconCharging: false
    property string title: ""
    property string subtitle: ""

    implicitWidth: Math.max(150,
        (root.iconType.length > 0 ? 30 : iconText.implicitWidth) + 14 + titleCol.implicitWidth + 48)
    implicitHeight: 56

    Txt {
        id: iconText
        visible: root.iconType.length === 0
        anchors.left: parent.left
        anchors.leftMargin: 24
        anchors.verticalCenter: parent.verticalCenter
        text: root.icon
        font.family: Theme.fontName
        font.pixelSize: 26
        color: Theme.fg
    }

    StatusIcon {
        visible: root.iconType.length > 0
        anchors.left: parent.left
        anchors.leftMargin: 22
        anchors.verticalCenter: parent.verticalCenter
        type: root.iconType
        level: root.iconLevel
        active: root.iconActive
        muted: root.iconMuted
        charging: root.iconCharging
        iconSize: 28
        color: Theme.fg
    }

    Column {
        id: titleCol
        anchors.left: iconText.right
        anchors.leftMargin: 14
        anchors.verticalCenter: parent.verticalCenter
        spacing: 2

        Txt {
            text: root.title
            font.family: Theme.fontName
            font.pixelSize: 15
            font.bold: true
            color: Theme.fg
        }

        Txt {
            text: root.subtitle
            font.family: Theme.fontName
            font.pixelSize: 12
            color: Theme.fg2
            visible: root.subtitle.length > 0
        }
    }
}