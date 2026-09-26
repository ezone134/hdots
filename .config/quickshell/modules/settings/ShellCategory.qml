import QtQuick
import "../../components"
import "../../services"

// Shell category: floating bar toggle and the auto-hide delay.
// Persisted under $states/settings.json "shell" via SettingsManager.set().
Column {
    id: root
    spacing: 16

    function hideToValue(ms) { return Math.max(0, Math.min(1, (ms - 500) / 9500)) }
    function valueToHide(v) { return Math.round(500 + v * 9500) }

    // Floating bar toggle
    Item {
        width: 440
        height: 44

        Column {
            anchors.left: parent.left
            anchors.right: floatingSwitch.left
            anchors.rightMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2

            Txt {
                text: "Floating bar"
                color: Theme.fg
                font.family: Theme.fontName
                font.pixelSize: 13
                font.bold: true
            }

            Txt {
                width: parent.width
                wrapMode: Text.WordWrap
                text: "On: the island floats below the top edge as a pill. Off: the same bar slides up flush against the top and takes a flatter, bar-like shape with smaller rounded corners (Mac Dynamic Island)."
                color: Theme.fg3
                font.family: Theme.fontName
                font.pixelSize: 11
            }
        }

        ToggleSwitch {
            id: floatingSwitch
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            checked: SettingsManager.config.shell.floating !== false
            onToggled: value => SettingsManager.set("shell", "floating", value)
        }
    }

    // Expand collapsed island → dashboard on hover. Off = simple compact bar.
    Item {
        width: 440
        height: 56

        Column {
            anchors.left: parent.left
            anchors.right: hoverExpandSwitch.left
            anchors.rightMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2

            Txt {
                text: "Expand on hover"
                color: Theme.fg
                font.family: Theme.fontName
                font.pixelSize: 13
                font.bold: true
            }

            Txt {
                width: parent.width
                wrapMode: Text.WordWrap
                text: "On: hover the island to open the full dashboard. Off: stay a simple compact bar (no hover expand) — open panels from chips or shortcuts instead."
                color: Theme.fg3
                font.family: Theme.fontName
                font.pixelSize: 11
            }
        }

        ToggleSwitch {
            id: hoverExpandSwitch
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            checked: SettingsManager.config.shell.expandOnHover !== false
            onToggled: value => {
                SettingsManager.set("shell", "expandOnHover", value)
                // Always-expanded mode is retired; force collapsed base state.
                if (SettingsManager.config.shell.expandedBar === true)
                    SettingsManager.set("shell", "expandedBar", false)
            }
        }
    }

    Txt {
        text: "Auto-hide delay: " + ((SettingsManager.config.shell.hideDelayMs || 3000) / 1000).toFixed(1) + "s"
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 11
    }

    Txt {
        width: 420
        wrapMode: Text.WordWrap
        text: "How long the island stays expanded before collapsing back to the pill (volume, brightness, and other transient overlays)."
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 11
    }

    Item {
        id: hideTrack
        width: 400
        height: 24

        Rectangle {
            anchors.fill: parent
            radius: height / 2
            color: Theme.bg2
        }

        Item {
            anchors.fill: parent
            anchors.margins: 3
            clip: false

            Rectangle {
                id: hideFill
                height: parent.height
                width: Math.max(height, parent.width * root.hideToValue(SettingsManager.config.shell.hideDelayMs || 3000))
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                radius: height / 2
                color: Theme.acc
            }
        }

        MouseArea {
            anchors.fill: parent
            onPressed: mouse => {
                SettingsManager.set("shell", "hideDelayMs", root.valueToHide(hideTrack.pctFromX(mouse.x)))
            }
            onPositionChanged: mouse => {
                if (pressed)
                    SettingsManager.set("shell", "hideDelayMs", root.valueToHide(hideTrack.pctFromX(mouse.x)))
            }
        }

        function pctFromX(x) {
            var p = (x - 3) / (width - 6)
            return Math.max(0, Math.min(1, p))
        }
    }
}
