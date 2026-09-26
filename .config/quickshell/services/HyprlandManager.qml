pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland
import QsHypr 1.0
import "."

// Event-driven Hyprland state. Instead of spawning hyprctl processes on an
// 800ms timer, we react to the Hyprland event socket (via the module's
// rawEvent signal): workspace/window events trigger a short debounced full
// refresh, focusedmon updates currentWorkspace instantly, and a 60s fallback
// heartbeat catches anything the event stream might miss. Full state snapshots
// (workspaces/clients) come from QsHypr — an in-process socket client, so no
// process is forked even when something changed.
Scope {
    id: root

    // Touches the module's live workspace model so it populates at startup
    // (the singleton is created lazily otherwise).
    property var moduleSeed: Hyprland.workspaces

    // Focused workspace tracks the reactive module property directly.
    // 0 = unknown (socket not connected). Using 1 as a fallback masks the
    // startup "1 -> 1" transition, which would skip the change signal and
    // leave the workspace OSD baseline unseeded (first switch silently eaten).
    property int currentWorkspace: Hyprland.focusedWorkspace && Hyprland.focusedWorkspace.id > 0
        ? Hyprland.focusedWorkspace.id : 0
    // Live focused toplevel from the reactive Quickshell module (no process).
    // class/title are resolved by matching the address against the parsed
    // windows (HyprlandToplevel has no `class` property, but the clients -j
    // snapshot does) — see findFocusedWindow().
    property var activeToplevel: Hyprland.activeToplevel
    property var focusedWindow: null
    property var workspaceIds: [1, 2]
    property var windowsByWorkspace: ({})
    property var workspaceNames: ({})
    property var allWindows: []
    property bool ready: false
    // Raw state snapshots fed by QsHypr (in-process, no hyprctl fork).
    property string workspacesRaw: ""
    property string clientsRaw: ""
    property int pendingRefresh: 0

    onActiveToplevelChanged: root.findFocusedWindow()

    // Events that change the client list (open/close/move/fullscreen/floating,
    // workspace/focus changes) need the clients + workspaces JSON re-read.
    property var clientEvents: ["openwindow", "closewindow", "movewindow", "openlayer", "closelayer", "fullscreen", "floating", "moveworkspace", "workspace", "renameworkspace", "urgent", "togglegroup", "movegroup", "focusedmon", "activewindow"]

    function ensureWorkspace(id) {
        const key = String(id);
        if (!windowsByWorkspace[key]) {
            const next = Object.assign({}, windowsByWorkspace);
            next[key] = [];
            windowsByWorkspace = next;
        }
    }

    function normalizeClassName(client) {
        const candidates = [client.class, client.initialClass, client.title, client.initialTitle];
        for (let i = 0; i < candidates.length; i++) {
            const value = String(candidates[i] ?? "").trim();
            if (value.length > 0)
                return value;
        }
        return "App";
    }

    function normalizeTitle(client) {
        const value = String(client.title ?? client.initialTitle ?? "").trim();
        return value.length > 0 ? value : normalizeClassName(client);
    }

    function normalizeAddress(client) {
        return String(client.address ?? "").trim();
    }

    function parseState() {
        let workspaces = [];
        let parsedClients = [];

        try {
            workspaces = JSON.parse(root.workspacesRaw || "[]");
        } catch (e) {
            workspaces = [];
        }

        try {
            parsedClients = JSON.parse(root.clientsRaw || "[]");
        } catch (e) {
            parsedClients = [];
        }

        const ids = [];
        const byWorkspace = {};
        const names = {};
        const windows = [];

        for (let i = 0; i < workspaces.length; i++) {
            const ws = workspaces[i];
            const id = Number(ws.id);
            if (!isNaN(id) && id > 0 && ids.indexOf(id) === -1)
                ids.push(id);
            if (!isNaN(id) && id > 0) {
                const name = String(ws.name ?? "").trim();
                if (name.length > 0)
                    names[String(id)] = name;
            }
        }

        for (let i = 0; i < parsedClients.length; i++) {
            const client = parsedClients[i];
            const workspaceId = Number(client.workspace && client.workspace.id);
            if (isNaN(workspaceId) || workspaceId <= 0)
                continue;

            if (ids.indexOf(workspaceId) === -1)
                ids.push(workspaceId);

            const key = String(workspaceId);
            if (!byWorkspace[key])
                byWorkspace[key] = [];

            const normalized = {
                address: normalizeAddress(client),
                title: normalizeTitle(client),
                className: normalizeClassName(client),
                workspaceId: workspaceId,
                mapped: client.mapped !== false,
                floating: client.floating === true,
                fullscreen: client.fullscreen === true
            };

            byWorkspace[key].push(normalized);
            windows.push(normalized);
        }

        ids.sort((a, b) => a - b);
        if (ids.length === 0)
            ids.push(1, 2);
        if (ids.indexOf(1) === -1)
            ids.unshift(1);
        if (ids.indexOf(2) === -1)
            ids.push(2);

        for (let i = 0; i < ids.length; i++) {
            const key = String(ids[i]);
            if (!byWorkspace[key])
                byWorkspace[key] = [];
        }

        workspaceIds = ids;
        workspaceNames = names;
        windowsByWorkspace = byWorkspace;
        allWindows = windows;
        root.findFocusedWindow();
        ready = true;
    }

    function refresh() {
        root.pendingRefresh = 2
        QsHypr.request("workspaces", true, function(text) {
            root.workspacesRaw = text
            root.refreshDone()
        })
        QsHypr.request("clients", true, function(text) {
            root.clientsRaw = text
            root.refreshDone()
        })
    }

    // Fires once both snapshots are in (avoids the old race where one
    // Process finished with stale data from the other).
    function refreshDone() {
        root.pendingRefresh -= 1
        if (root.pendingRefresh <= 0)
            root.parseState()
    }

    function windowsForWorkspace(id) {
        const key = String(id);
        return windowsByWorkspace[key] || [];
    }

    function focusWorkspace(id) {
        // 0.55 dispatchers are Lua: the old `dispatch workspace N` no longer
        // parses — use the hl.dsp.focus closure form (verified against 0.55.4).
        QsHypr.dispatch("hl.dsp.focus({workspace=" + String(id) + "})")
    }

    function focusWindow(address) {
        const trimmed = String(address ?? "").trim();
        if (trimmed.length === 0)
            return;
        QsHypr.dispatch("hl.dsp.focus({window='address:" + trimmed + "'})")
    }

    // Match the reactive activeToplevel address against the parsed window
    // snapshot so consumers get a normalized {address, title, className}
    // object (HyprlandToplevel exposes title but not class).
    function findFocusedWindow() {
        const addr = root.activeToplevel ? String(root.activeToplevel.address) : "";
        if (addr.length === 0) {
            root.focusedWindow = null;
            return;
        }
        const wins = root.allWindows;
        for (let i = 0; i < wins.length; i++) {
            if (String(wins[i].address) === addr) {
                root.focusedWindow = wins[i];
                return;
            }
        }
        root.focusedWindow = null;
    }

    // Nerd Font glyph for a window class (shared by the window switcher and
    // the pill's focused-window indicator).
    function appGlyph(className) {
        const c = String(className || "").toLowerCase()
        const glyphs = {
            "firefox": "\uf269",
            "chrome": "\uf268",
            "chromium": "\uf268",
            "brave": "\uf268",
            "vivaldi": "\uf268",
            "code": "\uf121",
            "vscodium": "\uf121",
            "codium": "\uf121",
            "cursor": "\uf121",
            "kitty": "\uf489",
            "alacritty": "\uf489",
            "wezterm": "\uf489",
            "ghostty": "\uf489",
            "foot": "\uf489",
            "konsole": "\uf489",
            "gnome-terminal": "\uf489",
            "terminal": "\uf489",
            "nautilus": "\uf07c",
            "thunar": "\uf07c",
            "dolphin": "\uf07c",
            "nemo": "\uf07c",
            "pcmanfm": "\uf07c",
            "spotify": "\uf1bc",
            "discord": "\uf6a7",
            "telegram": "\uf1d8",
            "slack": "\uf198",
            "steam": "\uf1b6",
            "gimp": "\uf1fc",
            "libreoffice": "\uf15c",
            "gedit": "\uf040",
            "kate": "\uf040",
            "gvim": "\uf040",
            "vim": "\uf040",
            "blender": "\uf1b2",
            "vlc": "\uf1c7",
            "mpv": "\uf008",
            "signal": "\uf5b7",
            "whatsapp": "\uf232",
            "obsidian": "\uf0c4"
        }
        return glyphs[c] || "\uf868"
    }

    function workspaceRowModel() {
        const rows = [];
        const ids = workspaceIds.length > 0 ? workspaceIds.slice() : [1, 2];
        for (let i = 0; i < ids.length; i++) {
            const id = ids[i];
            rows.push({
                workspaceId: id,
                windows: windowsForWorkspace(id)
            });
        }
        return rows;
    }

    // Debounced full refresh: coalesces bursts of socket events (e.g. a
    // window drag fires several movewindow events back to back) into one
    // batch of two QsHypr requests instead of one Process spawn per event.
    Timer {
        id: refreshDebounce
        interval: 60
        repeat: false
        onTriggered: root.refresh()
    }

    Connections {
        target: Hyprland
        function onRawEvent(event) {
            if (root.clientEvents.indexOf(event.name) !== -1)
                refreshDebounce.start()
        }
    }

    Component.onCompleted: root.refresh()
}