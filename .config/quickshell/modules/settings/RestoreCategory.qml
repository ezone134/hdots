import QtQuick
import "."
import "../../components"
import "../../services"

// Restore Defaults category (mirrors the "Restore Defaults" menu): wipes
// the state files for one channel or all channels, regenerates the flags
// and re-applies the theme. Destructive — every action needs confirmation.
Column {
    id: root
    anchors.fill: parent
    spacing: 12

    property string pendingRestore: ""

    ConfirmDialog {
        id: confirmDialog
    }

    Connections {
        target: confirmDialog
        function onConfirmed() { root.runRestore(root.pendingRestore) }
    }

    function runRestore(which) {
        const ch = SystemSettingsManager.channel
        const mo = SystemSettingsManager.mode
        const base = "luajit $sources/gen_flags_master; echo " + mo + " > $states2/m; echo " + ch + " > $states2/s; theme_main restore"
        if (which === "all")
            SystemSettingsManager.run("rm $states/* ; " + base)
        else
            SystemSettingsManager.run("rm $states/*_" + ch + " ; " + base)
    }

    function confirm(which) {
        root.pendingRestore = which
        if (which === "all")
            confirmDialog.ask("Restore Everything", "Wipes ALL state files (every channel) and restores defaults. This cannot be undone.", "Restore all")
        else
            confirmDialog.ask("Restore This Channel", "Wipes the state files of the active channel (" + SystemSettingsManager.channel + ") and restores its defaults.", "Restore")
    }

    Txt {
        text: "Restore settings"
        color: Theme.fg
        font.family: Theme.fontName
        font.pixelSize: 14
        font.bold: true
    }

    Txt {
        width: parent.width
        wrapMode: Text.WordWrap
        text: "Restores the hdots state files to their defaults and regenerates every config. Choose how much to wipe — the active channel only, or everything."
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 11
    }

    SettingsAction {
        label: "Only this channel"
        glyph: "\uf0c8"
        hint: "Active channel: " + SystemSettingsManager.channel
        onClicked: root.confirm("channel")
    }

    SettingsAction {
        label: "Everything / All channels"
        glyph: "\uf2d1"
        hint: "Wipes all state files"
        onClicked: root.confirm("all")
    }
}
