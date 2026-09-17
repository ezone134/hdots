pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import "."

Scope {
    id: root

    property var apps: []
    property bool loaded: false
    property bool loading: false

    function ensureLoaded() {
        if (!loaded && !loading)
            scanProc.running = true
    }

    function filter(text) {
        const query = String(text || "").trim().toLowerCase()
        if (query.length === 0)
            return apps

        return apps.filter(function(app) {
            return String(app.name || "").toLowerCase().indexOf(query) !== -1
                || String(app.id || "").toLowerCase().indexOf(query) !== -1
        })
    }

    // The scan bash resolves each Icon= value to a real path (pure bash,
    // no per-item existence checks). Quickshell 0.3.0 has no File.exists,
    // so this just wraps the stored path.
    function resolveIcon(iconName) {
        const icon = String(iconName || "").trim()
        if (icon.length === 0)
            return ""
        if (icon.indexOf("/") === 0)
            return "file://" + icon
        return ""
    }

    // Re-scan when the icon theme changes ($states/settings.json / SettingsPanel)
    // so stored icon paths stay in sync with the active theme.
    function rescan() {
        if (loading)
            return
        scanProc.running = false
        scanProc.running = true
    }

    function launch(appId) {
        const id = String(appId || "").trim()
        if (id.length === 0)
            return
        launchProc.appId = id
        launchProc.startDetached()
    }

    // NOT scanned at shell startup — the full .desktop scan is expensive and
    // slows first appearance. It happens lazily the first time a consumer
    // (launcher / command palette) calls ensureLoaded().
    Process {
        id: scanProc
        command: [
            "bash", "-c",
            "for dir in /usr/share/applications \"$HOME/.local/share/applications\"; do " +
            "  [ -d \"$dir\" ] || continue; " +
            "  for f in \"$dir\"/*.desktop; do " +
            "    [ -f \"$f\" ] || continue; " +
            "    id=${f##*/}; id=${id%.desktop}; " +
            "    name=; icon=; hidden=0; " +
            "    while IFS= read -r line; do " +
            "      case \"$line\" in " +
            "        'NoDisplay=true') hidden=1 ;; " +
            "        Name=*) [ -z \"$name\" ] && name=${line#Name=} ;; " +
            "        Icon=*) [ -z \"$icon\" ] && icon=${line#Icon=} ;; " +
            "      esac; " +
            "      [ \"$hidden\" -eq 1 ] && [ -n \"$name\" ] && [ -n \"$icon\" ] && break; " +
            "    done < \"$f\"; " +
            "    [ \"$hidden\" -eq 1 ] && continue; " +
            "    [ -n \"$name\" ] || continue; " +
            "    printf '%s\\t%s\\t%s\\n' \"$id\" \"$name\" \"$icon\"; " +
            "  done; " +
            "done"
        ]
        running: false
        onRunningChanged: loading = running
        stdout: SplitParser {
            onRead: line => {
                const parts = line.split("\t")
                if (parts.length >= 2) {
                    root.apps = root.apps.concat([{
                        id: parts[0].trim(),
                        name: parts[1].trim(),
                        icon: parts.length >= 3 ? parts[2].trim() : ""
                    }])
                }
            }
        }
        onStarted: root.apps = []
        onExited: {
            root.apps.sort(function(a, b) {
                return a.name.localeCompare(b.name)
            })
            root.loaded = true
        }
    }

    Process {
        id: launchProc
        property string appId: ""
        command: ["gtk-launch", appId]
        running: false
    }
}