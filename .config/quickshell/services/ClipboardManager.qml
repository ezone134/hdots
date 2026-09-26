pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import "."

// Clipboard history for text (default cliphist db) and images
// (~/.cache/cliphist_image/db), mirroring the user's existing rofi
// scripts: `cliphist list | cliphist decode | wl-copy`.
Scope {
    id: root

    readonly property string imageDbPath: Quickshell.env("HOME") + "/.cache/cliphist_image/db"

    property var textEntries: []
    property var imageEntries: []
    property bool textLoading: false
    property bool imageLoading: false
    property bool thumbsReady: false

    property var pendingText: []
    property var pendingImage: []

    signal copied()
    property string lastCopiedPreview: ""

    function previewOf(line) {
        let idx = String(line).indexOf("\t")
        let content = idx === -1 ? String(line) : String(line).slice(idx + 1)
        content = content.replace(/\n/g, " ")
        content = content.replace(/\s+/g, " ").trim()
        if (content.length > 42)
            content = content.slice(0, 42) + "…"
        return content
    }

    function refresh(mode) {
        if (mode === "image")
            root.refreshImages()
        else
            root.refreshText()
    }

    function refreshText() {
        if (root.textLoading)
            return
        root.textLoading = true
        root.pendingText = []
        listProc.mode = "text"
        listProc.command = ["cliphist", "list"]
        listProc.running = true
    }

    function refreshImages() {
        if (root.imageLoading)
            return
        root.imageLoading = true
        root.pendingImage = []
        root.thumbsReady = false
        listProc.mode = "image"
        listProc.command = ["cliphist", "-db-path", root.imageDbPath, "list"]
        listProc.running = true
        root.startThumbs()
    }

    function copy(line, mode) {
        copyProc.preview = root.previewOf(line)
        if (mode === "image") {
            copyProc.command = ["sh", "-c", "printf '%s\\n' \"$1\" | cliphist -db-path \"$2\" decode | wl-copy", "cliphist", line, root.imageDbPath]
        } else {
            copyProc.command = ["sh", "-c", "printf '%s\\n' \"$1\" | cliphist decode | wl-copy", "cliphist", line]
        }
        copyProc.running = true
    }

    function remove(line, mode) {
        if (mode === "image") {
            delProc.command = ["sh", "-c", "printf '%s\\n' \"$1\" | cliphist -db-path \"$2\" delete", "cliphist", line, root.imageDbPath]
        } else {
            delProc.command = ["sh", "-c", "printf '%s\\n' \"$1\" | cliphist delete", "cliphist", line]
        }
        delProc.mode = mode
        delProc.running = true
    }

    // Decode the most recent entries to /tmp so the picker can show
    // real thumbnails. One bash process instead of one per item; pure
    // bash (read + arithmetic + parameter expansion) — no cut/head.
    function startThumbs() {
        thumbProc.command = [
            "bash", "-c",
            "rm -f \"$1\"/*.img 2>/dev/null; mkdir -p \"$1\"; n=0; while IFS=$'\\t' read -r i rest; do ((n+=1)); [ \"$n\" -gt 60 ] && break; cliphist -db-path \"$2\" decode <<< \"$i\" > \"$1/$i.img\" 2>/dev/null; done < <(cliphist -db-path \"$2\" list)",
            "thumbs", "/tmp/cliphist_thumbs", root.imageDbPath
        ]
        thumbProc.running = true
    }

    function pushEntry(mode, line) {
        if (mode === "text")
            root.pendingText.push(line)
        else
            root.pendingImage.push(line)
    }

    // `cliphist list` entries with embedded newlines span multiple read
    // lines; a new entry only starts on a line beginning with index + tab.
    Process {
        id: listProc
        property string mode: ""
        property string pendingLine: ""

        stdout: SplitParser {
            onRead: line => {
                if (String(line).search(/^\d+\t/) === 0) {
                    if (listProc.pendingLine.length > 0)
                        root.pushEntry(listProc.mode, listProc.pendingLine)
                    listProc.pendingLine = String(line)
                } else if (listProc.pendingLine.length > 0) {
                    listProc.pendingLine += "\n" + String(line)
                }
            }
        }

        onExited: {
            if (listProc.pendingLine.length > 0)
                root.pushEntry(listProc.mode, listProc.pendingLine)
            listProc.pendingLine = ""
            if (listProc.mode === "text") {
                root.textLoading = false
                root.textEntries = root.pendingText
                root.pendingText = []
            } else {
                root.imageLoading = false
                root.imageEntries = root.pendingImage
                root.pendingImage = []
            }
        }
    }

    Process {
        id: copyProc
        property string preview: ""
        command: ["true"]
        onExited: {
            if (exitCode === 0 && copyProc.preview.length > 0) {
                root.lastCopiedPreview = copyProc.preview
                root.copied()
            }
            copyProc.preview = ""
        }
    }

    Process {
        id: delProc
        property string mode: ""
        command: ["true"]
        onExited: root.refresh(delProc.mode)
    }

    Process {
        id: thumbProc
        command: ["true"]
        onExited: root.thumbsReady = true
    }
}