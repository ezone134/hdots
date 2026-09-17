pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import "."

Scope {
    id: root

    property string wallpaperDir: Quickshell.env("HOME") + "/.cache/thumbnails/hdots/wall_thumbs"
    property var wallpapers: []
    property var pendingPaths: []

    function setWallpaper(path) {
        let parts = path.split("/")
        let basename = parts[parts.length - 1]
        let realPath = Quickshell.env("HOME") + "/Wallpapers/" + basename
        console.log("Applying wallpaper: " + realPath)
        wallpaperProc.running = false
        wallpaperProc.command = ["set_wall", realPath]
        wallpaperProc.running = true
    }

    function trigger() {
        console.log("Scanning: " + wallpaperDir)
        wallpapers = []
        pendingPaths = []
        scanProc.running = false
        scanProc.running = true
    }

    // Batch the array updates so the GridView only re-renders every 50ms
    // instead of on every single line from `find` (which caused jank).
    Timer {
        id: flushTimer
        interval: 50
        repeat: false
        onTriggered: {
            root.wallpapers = root.pendingPaths
            root.pendingPaths = []
        }
    }

    Process {
        id: scanProc
        // Pure bash scan (globstar) — no find subprocess.
        command: [
            "bash", "-c",
            "shopt -s globstar nullglob; for f in \"$1\"/**/*; do [ -f \"$f\" ] && printf '%s\\n' \"$f\"; done",
            "scan", wallpaperDir
        ]
        stdout: SplitParser {
            onRead: line => {
                let path = line.trim()
                if (path.length > 0) {
                    root.pendingPaths.push(path)
                    flushTimer.restart()
                }
            }
        }
        onExited: {
            flushTimer.stop()
            root.wallpapers = root.pendingPaths
            root.pendingPaths = []
            console.log("Scan complete. Items found: " + root.wallpapers.length)
        }
    }

    Process {
        id: wallpaperProc
        command: ["set_wall", ""]
    }
}