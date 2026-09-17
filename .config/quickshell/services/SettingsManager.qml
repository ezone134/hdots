pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import "."
import "../components"

// Persistent shell settings, stored as JSON in the state dir
// ($states/settings.json). Changes apply live and are written back
// so the file is always the source of truth; the file is watched, so
// hand-editing it applies without a restart.
Scope {
    id: root

    readonly property string configPath: SystemSettingsManager.states + "/settings.json"
    property var config: ({
        "appearance": {
            "accent": "#183048",
            "theme": "dark",
            "font": Theme.fontName,
            "iconTheme": "Tela-nord-dark"
        },
        "notifications": {
            "dnd": false,
            "timeoutMs": 5000,
            "maxVisible": 4,
            "maxHistory": 50,
            "anchor": "top-right",
            "width": 300,
            "blockedApps": [],
            "filters": {
                "login_splash": true,
                "caps_lock": true,
                "audio_pop": true,
                "theme": true,
                "theme_extras": true,
                "opacity": true,
                "shader": true,
                "hyprsunset": true,
                "shadow": true,
                "blur": true,
                "battery": true,
                "clip": true,
                "extractor": true,
                "symlink": true,
                "wallpaper": true,
                "kill_mode": true,
                "brightness": true,
                "recording": true,
                "system": true
            }
        },
        "shell": {
            "hideDelayMs": 3000,
            "floating": true,
            "expandedBar": false,
            "expandOnHover": true,
            "settingsFullscreen": false
        },
        "pill": {
            "collapsedBattery": false,
            "collapsedWatts": false,
            "collapsedWeather": false,
            "collapsedNotification": false,
            "collapsedWorkspace": false,
            "collapsedSettings": false,
            "collapsedPower": false,
            "musicBarAnim": true,
            "itemOrder": [
                "clock",
                "musicBarAnim",
                "collapsedBattery",
                "collapsedWatts",
                "collapsedWeather",
                "collapsedNotification",
                "collapsedWorkspace",
                "collapsedSettings",
                "collapsedPower"
            ]
        },
        "capture": {
            "screenshotDir": "",
            "recordDir": "",
            "format": "png",
            "copyScreenshot": false,
            "fps": 60
        },
        "audioRecorder": {
            "confirmDelete": true,
            "showMicSlider": true
        }
    })
    property bool ready: false

    readonly property var sections: ["appearance", "notifications", "shell", "pill", "capture", "audioRecorder"]

    function mergeSection(fileCfg, section) {
        const base = Object.assign({}, root.config[section] || {}, fileCfg[section] || {})
        // Deep-merge nested objects so partial settings.json doesn't wipe defaults
        // (e.g. notifications.filters keys).
        if (section === "notifications") {
            const defF = (root.config.notifications && root.config.notifications.filters) || {}
            const fileF = (fileCfg.notifications && fileCfg.notifications.filters) || {}
            base.filters = Object.assign({}, defF, fileF)
        }
        return base
    }

    function parse(text) {
        let fileCfg = {}
        try {
            fileCfg = JSON.parse(String(text || "") || "{}")
        } catch (e) {
            fileCfg = {}
        }

        const merged = {}
        for (let i = 0; i < root.sections.length; i++) {
            const s = root.sections[i]
            merged[s] = root.mergeSection(fileCfg, s)
        }
        root.config = merged
        root.apply()
        root.ready = true
    }

    // Rebuild the top-level object so QML bindings see a change, then
    // apply live; the write is debounced so rapid successive changes
    // collapse into one save of the final state (no stale-write races
    // with the file watcher reloading mid-sequence). Idempotent.
    function set(section, key, value) {
        const next = {}
        for (let i = 0; i < root.sections.length; i++) {
            const s = root.sections[i]
            next[s] = Object.assign({}, root.config[s] || {})
        }
        next[section] = Object.assign({}, next[section], {})
        next[section][key] = value
        root.config = next
        root.apply()
        saveDebounce.restart()
    }

    Timer {
        id: saveDebounce
        interval: 50
        onTriggered: root.save()
    }

    function save() {
        saveLauncher.launch([
            "bash", "-c",
            "mkdir -p \"${1%/*}\" && printf '%s\\n' \"$2\" > \"$1\"",
            "cfg", root.configPath, JSON.stringify(root.config, null, 2)
        ])
    }

    property DetachedLauncher saveLauncher: DetachedLauncher {}

    function apply() {
        const app = root.config.appearance || {}
        const notif = root.config.notifications || {}
        const shell = root.config.shell || {}

        Theme.applyConfig(app)

        NotificationManager.setDnd(notif.dnd === true)
        NotificationManager.applyBlockedApps(notif.blockedApps)
        NotificationManager.applyFilters(notif.filters)
        NotificationManager.applySettings()

        if (shell.hideDelayMs)
            StateController.hideTimer.interval = Number(shell.hideDelayMs)
    }


    // FileView is used only as a change signal: its text() cache can be
    // stale at signal time and empty at startup (async load). Every reload
    // reads the file fresh from disk so parse() always sees current data.
    function reload() {
        loadRunner.command = [
            "bash", "-c", "cat \"$1\" 2>/dev/null", "cfg", root.configPath
        ]
        loadRunner.run()
    }

    property CommandRunner loadRunner: CommandRunner {
        onFinished: root.parse(loadRunner.text)
    }

    FileView {
        id: changeWatcher
        path: root.configPath
        watchChanges: true
        blockLoading: false
        onFileChanged: root.reload()
    }

    Component.onCompleted: {
        root.ready = true
        root.reload()
    }
}