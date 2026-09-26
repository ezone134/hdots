import QtQuick
import "."
import "../../components"
import "../../services"

// Scrollable settings section: owns a SettingEngine for the requested
// schema section(s) and renders every row as a SettingItem delegate.
// Reads happen once when the section appears and again whenever the target
// section list or the active channel/mode changes — never on a timer, so a
// closed section costs nothing. When a combo expands near the bottom edge
// the view auto-scrolls so the list stays visible.
Item {
    id: root

    property var sections: []
    property string defaultScope: "channel"
    property string schemaFile: ""
    property alias engine: sectionEngine

    signal writeDone(string key)

    SettingEngine {
        id: sectionEngine
        sections: root.sections
        defaultScope: root.defaultScope
        schemaFile: root.schemaFile
        onWriteDone: key => root.writeDone(key)
    }

    Flickable {
        id: sectionFlick
        anchors.fill: parent
        clip: true
        contentHeight: column.height
        boundsBehavior: Flickable.StopAtBounds

        Behavior on contentY {
            NumberAnimation { duration: 180; easing.type: Easing.OutCubic }
        }

        Column {
            id: column
            width: sectionFlick.width
            spacing: 4

            Repeater {
                id: repeater
                model: sectionEngine.rows

                delegate: SettingItem {
                    required property var modelData
                    width: column.width
                    row: modelData
                    engine: sectionEngine
                    onComboOpened: root.ensureVisible(this)
                }
            }

            Item { width: 1; height: 14 }
        }
    }

    ScrollDragger {
        target: sectionFlick
        thumbWidth: 5
        minThumbHeight: 30
    }

    // Loading / error / empty overlay while the engine has no rows.
    readonly property string _state: {
        if (sectionEngine.error)
            return "error"
        if (!sectionEngine.ready)
            return "loading"
        if (sectionEngine.rows.length === 0)
            return "empty"
        return "rows"
    }

    Item {
        anchors.fill: parent
        visible: root._state !== "rows"
        enabled: false

        EmptyState {
            anchors.centerIn: parent
            icon: root._state === "loading" ? "\uf110"
                : root._state === "error" ? "\uf071"
                : "\uf05e"
            text: root._state === "loading" ? "Loading settings…"
                : root._state === "error" ? ("Settings schema error — " + sectionEngine.error)
                : "No settings in this section"
        }
    }

    // ---- auto-scroll a just-opened combo into view ----

    property var _ensureTarget: null

    function ensureVisible(item) {
        root._ensureTarget = item
        ensureTimer.restart()
    }

    Timer {
        id: ensureTimer
        interval: 20
        onTriggered: {
            const it = root._ensureTarget
            if (!it || !it.height)
                return
            const need = it.y + it.height + 8 - sectionFlick.height
            if (need > sectionFlick.contentY)
                sectionFlick.contentY = Math.max(0, need)
        }
    }

    // ---- reload triggers ----

    // Channel-scoped rows resolve different state files per channel/mode,
    // so an active channel change rebuilds this section's rows.
    Connections {
        target: SystemSettingsManager
        function onChannelChanged() { sectionEngine.load() }
        function onModeChanged() { sectionEngine.load() }
    }

    onSectionsChanged: sectionEngine.load()

    Component.onCompleted: sectionEngine.load()
}
