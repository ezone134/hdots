import QtQuick
import "../../components"
import "../../services"

// About category: shell info and the $states/settings.json location.
Column {
    spacing: 12

    Txt {
        text: "Quickshell Bar"
        color: Theme.fg
        font.family: Theme.fontName
        font.pixelSize: 18
        font.bold: true
    }

    Txt {
        text: "Quickshell 0.3.0 • Hyprland"
        color: Theme.fg2
        font.family: Theme.fontName
        font.pixelSize: 13
    }

    Txt {
        text: "Config: " + SettingsManager.configPath
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 12
        elide: Text.ElideMiddle
        width: 480
    }

    Txt {
        width: 480
        wrapMode: Text.WordWrap
        text: "Settings are saved live to $states/settings.json — edit the file anytime and changes apply automatically (the file is watched)."
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 12
    }

    Txt {
        text: "Keyboard: Up/Down or Left/Right switch category • Esc closes"
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 11
    }
}
