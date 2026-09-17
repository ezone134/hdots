import QtQuick
import "."
import "../../components"
import "../../services"

// Clipboard category (mirrors the "Clipboard Manager" menu): open the
// quickshell text/image history panels, or clear the cliphist databases
// behind a confirm dialog.
Column {
    id: root
    anchors.fill: parent
    spacing: 10

    property string pendingCmd: ""

    ConfirmDialog {
        id: confirmDialog
    }

    Connections {
        target: confirmDialog
        function onConfirmed() { SystemSettingsManager.run(root.pendingCmd) }
    }

    function clear(kind) {
        if (kind === "image") {
            root.pendingCmd = "rm -f \"$HOME/.cache/cliphist_image/db\"; notify-send -a Clipboard Clipboard \"Image history cleared\""
            confirmDialog.ask("Clear Image Data", "Delete the whole image clipboard database (~/.cache/cliphist_image/db)?", "Clear")
        } else {
            root.pendingCmd = "rm -f \"$HOME/.cache/cliphist/db\"; notify-send -a Clipboard Clipboard \"Text history cleared\""
            confirmDialog.ask("Clear Text Data", "Delete the whole text clipboard database (~/.cache/cliphist/db)?", "Clear")
        }
    }

    Txt {
        text: "History"
        color: Theme.fg2
        font.family: Theme.fontName
        font.pixelSize: 11
        font.bold: true
    }

    SettingsAction {
        label: "Open Text History"
        glyph: "\uf0ea"
        hint: "cliphist text list (Super+V)"
        onClicked: StateController.clipboard()
    }

    SettingsAction {
        label: "Open Image History"
        glyph: "\uf0c7"
        hint: "cliphist image list (Super+Shift+V)"
        onClicked: StateController.clipboardImages()
    }

    Txt {
        text: "Data"
        color: Theme.fg2
        font.family: Theme.fontName
        font.pixelSize: 11
        font.bold: true
    }

    SettingsAction {
        label: "Clear Text Data"
        glyph: "\uf12d"
        hint: "Deletes ~/.cache/cliphist/db"
        onClicked: root.clear("text")
    }

    SettingsAction {
        label: "Clear Image Data"
        glyph: "\uf12d"
        hint: "Deletes ~/.cache/cliphist_image/db"
        onClicked: root.clear("image")
    }
}
