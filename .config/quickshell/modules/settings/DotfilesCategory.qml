import QtQuick
import "."
import "../../components"
import "../../services"

// Generic schema-driven dotfiles category: channel/mode switcher on top,
// then the requested settings-schema.json section's rows (SettingSection)
// below. The section reloads whenever the channel or mode changes so
// channel-scoped rows point at the right state files.
Item {
    id: root

    property var sections: []
    property string defaultScope: "channel"

    Column {
        anchors.fill: parent
        spacing: 10

        ChannelModeSwitcher {
            id: chSwitcher
            width: parent.width
        }

        Rectangle {
            width: parent.width
            height: 1
            color: Theme.bg3
        }

        SettingSection {
            width: parent.width
            height: parent.height - chSwitcher.height - 21
            sections: root.sections
            defaultScope: root.defaultScope
        }
    }
}
