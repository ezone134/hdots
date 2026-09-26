pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Services.Notifications
import "."
import "../components/TextParsers.js" as TextParsers

Scope {
    id: root

    property var notifications: []
    property var history: []
    property int nextNotificationId: 1
    property int maxVisible: 4
    property int maxHistory: 50
    property int defaultTimeout: 5000
    property bool keepOnReload: true
    property bool dndEnabled: false
    property int unreadCount: 0
    // Apps the user has muted. Persisted as notifications.blockedApps in
    // $states/settings.json; a blocked app's notifications are dismissed on arrival
    // and never reach the toast stack or history.
    property var blockedApps: []

    // Category filters (notifications.filters in settings.json). Scripts always
    // emit notify-send; QS drops muted categories here. Keys match the Popups
    // toggles plus a few hypr_bin utilities.
    property var filters: ({})

    // Map notify-send app names (and a few summary fallbacks) → filter key.
    readonly property var filterAppMap: ({
        "theme": "theme",
        "Theme": "theme",
        "theme-extras": "theme_extras",
        "Theme extras": "theme_extras",
        "Theme Switcher": "theme_extras",
        "scrim_mode": "theme_extras",
        "init_defined_hex": "theme_extras",
        "init_custom_accent": "theme_extras",
        "init_hex_from_wall": "theme_extras",
        "gen_custom_acc_files": "theme_extras",
        "Shader": "shader",
        "shader": "shader",
        "Opacity": "opacity",
        "opacity": "opacity",
        "Blur": "blur",
        "blur": "blur",
        "Shadow": "shadow",
        "shadow": "shadow",
        "Hyprsunset": "hyprsunset",
        "hyprsunset": "hyprsunset",
        "Eye care": "hyprsunset",
        "Battery": "battery",
        "battery": "battery",
        "Clipboard": "clip",
        "clip": "clip",
        "Extractor": "extractor",
        "extractor": "extractor",
        "Symlink": "symlink",
        "symlink": "symlink",
        "Wallpaper": "wallpaper",
        "set_wall": "wallpaper",
        "awww-init": "wallpaper",
        "Kill Mode": "kill_mode",
        "kill_mode": "kill_mode",
        "Brightness": "brightness",
        "Brightness (Lazy)": "brightness",
        "Recording": "recording",
        "recording": "recording",
        "System": "system",
        "system": "system",
        "hypridle": "system",
        "Synced": "system",
        "caps_lock": "caps_lock",
        "Caps Lock": "caps_lock",
        "audio": "audio_pop",
        "Audio": "audio_pop",
        "login_splash": "login_splash",
        "Login splash": "login_splash"
    })

    function _decodeDbusString(value) {
        if (value === undefined || value === null)
            return "";

        let text = String(value).trim();

        if (text.startsWith("variant"))
            text = text.slice(7).trim();

        if (text.startsWith("string "))
            text = text.slice(7).trim();

        if (text.length >= 2 && text.startsWith("\"") && text.endsWith("\""))
            text = text.slice(1, -1);

        return text
            .replace(/\\n/g, "\n")
            .replace(/\\t/g, "\t")
            .replace(/\\\"/g, '"')
            .replace(/\\\\/g, "\\");
    }

    function _sanitizeMarkup(text) {
        return TextParsers.stripTags(text)
    }

    // Toast/history card style, derived from the shell theme + settings.
    // Kept as a plain object so history cards (NotificationPanel) render
    // identically to live toasts.
    function _style() {
        const n = (SettingsManager.config && SettingsManager.config.notifications) || {}
        return {
            backgroundColor: Theme.bg,
            borderColor: Theme.border,
            textColor: Theme.fg,
            fontFamily: Theme.fontName,
            borderRadius: 14,
            borderSize: 1,
            width: Number(n.width || 300),
            height: 500,
            fontPixelSize: 13,
            titleFontPixelSize: 14,
            appFontPixelSize: 12,
            bodyFontPixelSize: 13,
            progressColor: Theme.acc,
            textAlignment: "left",
            margin: 8
        }
    }

    // Re-read the notification limits from $states/settings.json and enforce the
    // caps on the currently-shown lists. Idempotent.
    function applySettings() {
        const n = (SettingsManager.config && SettingsManager.config.notifications) || {}
        if (Number(n.timeoutMs) > 0)
            root.defaultTimeout = Number(n.timeoutMs)
        if (Number(n.maxVisible) > 0)
            root.maxVisible = Number(n.maxVisible)
        if (Number(n.maxHistory) > 0)
            root.maxHistory = Number(n.maxHistory)
        root._capVisible()
        root.history = root.history.slice(0, root.maxHistory)
        root._refreshUnread()
    }

    function _refreshUnread() {
        root.unreadCount = 0
        for (let i = 0; i < root.history.length; i++) {
            if (!root.history[i].read)
                root.unreadCount++
        }
    }

    function _dismissItem(item) {
        if (!item || item.closed)
            return
        item.closed = true
        if (item.notification) {
            try { item.notification.dismiss() } catch (e) {}
        }
    }

    // Enforce maxVisible: anything past the cap is dropped from the on-screen
    // list (but stays in history). Dropped toasts are told to expire so the
    // source app sees a clean close.
    function _capVisible() {
        if (root.notifications.length <= root.maxVisible)
            return
        const dropped = root.notifications.slice(root.maxVisible)
        root.notifications = root.notifications.slice(0, root.maxVisible)
        for (let i = 0; i < dropped.length; i++) {
            const item = dropped[i]
            if (!item.closed) {
                item.closed = true
                if (item.notification) {
                    try { item.notification.expire() } catch (e) {}
                }
            }
        }
    }

    // Remove from the on-screen list; the underlying DBus notification is
    // dismissed so the source app gets the close signal.
    function removeNotification(id) {
        const kept = []
        for (let i = 0; i < notifications.length; i++) {
            const item = notifications[i]
            if (item.id !== id) {
                kept.push(item)
            } else {
                root._dismissItem(item)
            }
        }
        notifications = kept
    }

    function expireNotification(id) {
        removeNotification(id)
    }

    function markAllRead() {
        unreadCount = 0
        const updated = []
        for (let i = 0; i < history.length; i++) {
            const item = history[i]
            item.read = true
            updated.push(item)
        }
        history = updated
    }

    function removeHistoryNotification(id) {
        const filtered = []
        for (let i = 0; i < history.length; i++) {
            const item = history[i]
            if (item.id !== id)
                filtered.push(item)
        }
        history = filtered
    }

    function clearAll() {
        // Clears the whole notification store: the live toast stack, the
        // history list the panel renders, and the unread badge. Every item
        // is dismissed so the source app gets a clean close signal.
        const removed = notifications.concat(history)
        notifications = []
        history = []
        unreadCount = 0
        for (let i = 0; i < removed.length; i++)
            root._dismissItem(removed[i])
    }

    function clearHistory() {
        history = []
        unreadCount = 0
    }

    function toggleDnd() {
        dndEnabled = !dndEnabled
        if (!dndEnabled)
            notifications = history.slice(0, maxVisible)
    }

    // Idempotent setter so SettingsManager can sync the persisted value
    // on startup and on $states/settings.json changes.
    function setDnd(enabled) {
        if (root.dndEnabled === !!enabled)
            return
        root.dndEnabled = !!enabled
        if (!root.dndEnabled)
            notifications = history.slice(0, maxVisible)
    }

    function isAppBlocked(appName) {
        return root.blockedApps.indexOf(appName) !== -1
    }

    // Sync from $states/settings.json (called by SettingsManager.apply) — idempotent.
    function applyBlockedApps(list) {
        root.blockedApps = Array.isArray(list) ? list.slice() : []
    }

    function applyFilters(obj) {
        root.filters = (obj && typeof obj === "object") ? Object.assign({}, obj) : {}
    }

    function filterKeyFor(appName, summary) {
        const a = String(appName || "").trim()
        const s = String(summary || "").trim()
        if (root.filterAppMap[a] !== undefined)
            return root.filterAppMap[a]
        if (root.filterAppMap[s] !== undefined)
            return root.filterAppMap[s]
        const al = a.toLowerCase()
        for (const k in root.filterAppMap) {
            if (String(k).toLowerCase() === al)
                return root.filterAppMap[k]
        }
        return ""
    }

    // true → drop this notification (filter off). Unknown keys pass through.
    function isCategoryMuted(appName, summary) {
        const key = root.filterKeyFor(appName, summary)
        if (!key || !root.filters)
            return false
        if (root.filters[key] === undefined)
            return false
        return root.filters[key] !== true && root.filters[key] !== 1 && root.filters[key] !== "1"
    }

    function setFilter(key, enabled) {
        const k = String(key || "").trim()
        if (!k.length)
            return
        const next = Object.assign({}, root.filters)
        next[k] = !!enabled
        root.filters = next
        SettingsManager.set("notifications", "filters", Object.assign({}, next))
    }

    function toggleFilter(key) {
        const cur = root.filters && root.filters[key]
        root.setFilter(key, !(cur === true || cur === 1 || cur === "1"))
    }

    function _persistBlocked() {
        SettingsManager.set("notifications", "blockedApps", root.blockedApps.slice())
    }

    function blockApp(appName) {
        const name = String(appName || "").trim()
        if (name.length === 0 || root.isAppBlocked(name))
            return
        const next = root.blockedApps.slice()
        next.push(name)
        root.blockedApps = next
        root._persistBlocked()

        // Purge this app's live toasts and history so blocking takes effect
        // immediately and the panel doesn't keep stale entries.
        const kept = []
        for (let i = 0; i < root.history.length; i++) {
            const item = root.history[i]
            if (item.appName === name)
                root._dismissItem(item)
            else
                kept.push(item)
        }
        root.history = kept
        const visible = []
        for (let i = 0; i < root.notifications.length; i++) {
            const item = root.notifications[i]
            if (item.appName === name)
                root._dismissItem(item)
            else
                visible.push(item)
        }
        root.notifications = visible
        root._refreshUnread()
    }

    function unblockApp(appName) {
        const idx = root.blockedApps.indexOf(appName)
        if (idx === -1)
            return
        const next = root.blockedApps.slice()
        next.splice(idx, 1)
        root.blockedApps = next
        root._persistBlocked()
    }

    function toggleBlockedApp(appName) {
        if (root.isAppBlocked(appName))
            root.unblockApp(appName)
        else
            root.blockApp(appName)
    }

    function groupedHistory() {
        const groups = []
        const seen = {}
        for (let i = 0; i < history.length; i++) {
            const item = history[i]
            const key = (item.appName || "Unknown") + "::" + (item.category || "")
            if (!seen[key]) {
                seen[key] = {
                    appName: item.appName || "Unknown",
                    category: item.category || "",
                    items: []
                }
                groups.push(seen[key])
            }
            seen[key].items.push(item)
        }
        return groups
    }

    // Resolve the auto-dismiss time. Only critical notifications stay
    // until dismissed (0 = no timer). A negative timeout (-1) means "let
    // the server decide", so it falls back to the default timeout too.
    function _resolveTimeout(timeoutMs, urgency) {
        if (urgency === "critical")
            return 0
        if (timeoutMs > 0)
            return Number(timeoutMs)
        return root.defaultTimeout
    }

    function addNotification(appName, appIcon, summary, body, timeoutMs, urgency, category, actions, hasInlineReply, inlineReplyPlaceholder, notification) {
        const cleanAppName = _sanitizeMarkup(appName);
        const cleanSummary = _sanitizeMarkup(summary);
        const cleanBody = _sanitizeMarkup(body);
        const normalizedUrgency = urgency || "normal";

        // Per-app mute + category filters: dismiss on arrival, never toast/store.
        if (root.isAppBlocked(cleanAppName) || root.isCategoryMuted(cleanAppName, cleanSummary)) {
            if (notification) {
                try { notification.dismiss() } catch (e) {}
            }
            return
        }

        // Map DBus actions to plain objects with an invoke() closure. The
        // special "inline-reply" action is extracted (Quickshell surfaces it
        // separately on the notification object).
        const mappedActions = []
        let replyFlag = !!hasInlineReply
        let replyPlaceholder = String(inlineReplyPlaceholder || "")
        const srcActions = actions || []
        for (let i = 0; i < srcActions.length; i++) {
            const a = srcActions[i]
            const actionId = String((a && a.identifier) || "")
            if (actionId === "inline-reply") {
                replyFlag = true
                if (replyPlaceholder.length === 0)
                    replyPlaceholder = String((a && a.text) || "Reply")
            } else if (actionId.length > 0) {
                const label = String((a && a.text) || actionId)
                mappedActions.push({
                    identifier: actionId,
                    text: label,
                    invoke: function() { try { a.invoke() } catch (e) {} }
                })
            }
        }

        const item = {
            id: nextNotificationId++,
            appName: cleanAppName,
            appIcon: appIcon || "",
            image: (notification && notification.image) || "",
            summary: cleanSummary.length > 0 ? cleanSummary : cleanAppName,
            body: cleanBody,
            timeoutMs: root._resolveTimeout(Number(timeoutMs || 0), normalizedUrgency),
            urgency: normalizedUrgency,
            category: category || "",
            actions: mappedActions,
            hasInlineReply: replyFlag,
            inlineReplyPlaceholder: replyPlaceholder,
            notification: notification || null,
            closed: false,
            style: root._style(),
            timestamp: Date.now(),
            read: false
        };

        root.applySettings()
        root.history = [item].concat(root.history).slice(0, root.maxHistory);
        if (!root.dndEnabled)
            root.notifications = [item].concat(root.notifications).slice(0, root.maxVisible);
        root._capVisible()
        root._refreshUnread()
    }

    function show(title, message, icon) {
        addNotification("Quickshell", icon, title, message, 3000, "low", "", [], false, "")
    }

    function parseNotificationLine(line) {
        const parts = String(line ?? "").split("\u001f");
        if (parts.length < 6)
            return;

        addNotification(
            _decodeDbusString(parts[0]),
            _decodeDbusString(parts[1]),
            _decodeDbusString(parts[2]),
            _decodeDbusString(parts[3]),
            Number(parts[4]),
            _decodeDbusString(parts[5]),
            ""
        );
    }

    Connections {
        target: SettingsManager
        function onConfigChanged() { root.applySettings() }
    }

    NotificationServer {
        id: notificationServer
        actionsSupported: true
        inlineReplySupported: true
        imageSupported: true

        onNotification: notification => {
            const urgencyMap = {
                0: "low",
                1: "normal",
                2: "critical"
            }
            root.addNotification(
                notification.appName || "",
                notification.appIcon || "",
                notification.summary || "",
                notification.body || "",
                Number(notification.expireTimeout),
                urgencyMap[notification.urgency] || "normal",
                notification.category || "",
                notification.actions || [],
                notification.hasInlineReply || false,
                notification.inlineReplyPlaceholder || "",
                notification
            )
        }
    }
}