pragma ComponentBehavior: Bound
import QtQuick
import Quickshell
import "."
import "../../components"
import "../../services"
import "../themes"

// Full settings app: category nav on the left, live-editable controls on
// the right. Each category is its own bespoke file in this module.
// Changes go through SettingsManager.set() (persists to $states/settings.json).
Item {
    id: root
    focus: true

    // Fullscreen settings: when shell.settingsFullscreen is on, the panel
    // reports the screen size as its implicit size, so the whole morph chain
    // (container → surface → window) grows to cover the display. Persisted
    // in $states/settings.json, so the choice is remembered across restarts.
    readonly property bool fullscreen: SettingsManager.config.shell.settingsFullscreen === true
    // Not readonly: QsWindow.window may be null the instant the Loader
    // creates this panel (before it attaches to the window), and readonly
    // bindings never re-evaluate — screenW/H would stay 0 and fullscreen
    // would silently do nothing. A plain binding re-runs once the window
    // attaches. (QtQuick's Window.window is null inside Quickshell panels;
    // QsWindow is the Quickshell attached property.)
    property var panelWindow: QsWindow.window
    readonly property var panelScreen: root.panelWindow && root.panelWindow.screen ? root.panelWindow.screen : null
    readonly property real screenW: root.panelScreen && root.panelScreen.width > 0 ? root.panelScreen.width : 0
    readonly property real screenH: root.panelScreen && root.panelScreen.height > 0 ? root.panelScreen.height : 0

    implicitWidth: root.fullscreen && root.screenW > 0 ? root.screenW : 720
    implicitHeight: root.fullscreen && root.screenH > 0 ? root.screenH : 430

    property var categories: [
        { key: "appearance", icon: "\uf1fc", title: "Appearance", comp: "appearance" },
        { key: "notifications", icon: "\uf0f3", title: "Notifications", comp: "notification" },
        { key: "wallpaper", icon: "\uf03e", title: "Wallpaper", comp: "wallpaper" },
        { key: "shell", icon: "\uf013", title: "Shell", comp: "shell" },
        { key: "pill", icon: "\uf0c9", title: "Pill", comp: "pill" },
        { key: "keyboard", icon: "\uf11c", title: "Keyboard", comp: "keyboard" },
        { key: "keybinds", icon: "\uf11c", title: "Keybinds", comp: "keybinds" },
        { key: "capture", icon: "\uf03e", title: "Capture", comp: "capture" },
        { key: "wm", icon: "\uf108", title: "Window Manager", comp: "wm" },
        { key: "themes", icon: "\uf53f", title: "Themes", comp: "themes" },
        { key: "about", icon: "\uf05a", title: "About", comp: "about" }
    ]
    property int selectedCategory: 0

    readonly property var currentCategory: root.categories[root.selectedCategory] || null

    // One Component per bespoke category (matches PanelRegistry pattern).
    property Component appearanceCategory: Component { AppearanceCategory {} }
    property Component notificationCategory: Component { NotificationCategory {} }
    property Component wallpaperCategory: Component { WallpaperCategory {} }
    property Component shellCategory: Component { ShellCategory {} }
    property Component pillCategory: Component { PillCategory {} }
    property Component keyboardCategory: Component { KeyboardCategory {} }
    property Component keybindsCategory: Component { KeybindsCategory {} }
    property Component captureCategory: Component { CaptureCategory {} }
    property Component wmCategory: Component { WmCategory {} }
    property Component themesCategory: Component { ThemesPanel {} }
    property Component aboutCategory: Component { AboutCategory {} }

    function categoryComp(name) {
        if (name === "appearance") return root.appearanceCategory
        if (name === "notification") return root.notificationCategory
        if (name === "wallpaper") return root.wallpaperCategory
        if (name === "shell") return root.shellCategory
        if (name === "pill") return root.pillCategory
        if (name === "keyboard") return root.keyboardCategory
        if (name === "keybinds") return root.keybindsCategory
        if (name === "capture") return root.captureCategory
        if (name === "wm") return root.wmCategory
        if (name === "themes") return root.themesCategory
        if (name === "about") return root.aboutCategory
        return null
    }

    function clampCategory(index) {
        return Math.max(0, Math.min(index, root.categories.length - 1))
    }

    // When targeted via IPC (e.g. `keyboard_settings`), jump straight to the
    // requested category and clear the flag. Handled on creation AND live:
    // if Settings is already open, StateController just flips the flag and
    // this Connections handler re-targets without reopening.
    function applySettingsCategory() {
        const target = StateController.settingsCategory
        if (target.length === 0)
            return
        for (let i = 0; i < root.categories.length; i++) {
            if (root.categories[i].key === target) {
                root.selectedCategory = i
                break
            }
        }
        StateController.settingsCategory = ""
    }

    Component.onCompleted: root.applySettingsCategory()

    Connections {
        target: StateController
        function onSettingsCategoryChanged() { root.applySettingsCategory() }
    }

    OverlayFocusScope {
        focusTarget: root
    }

    Keys.onPressed: function(event) {
        if (event.key === Qt.Key_Up) {
            root.selectedCategory = root.clampCategory(root.selectedCategory - 1)
            event.accepted = true
        } else if (event.key === Qt.Key_Down) {
            root.selectedCategory = root.clampCategory(root.selectedCategory + 1)
            event.accepted = true
        } else if (event.key === Qt.Key_Left) {
            root.selectedCategory = root.clampCategory(root.selectedCategory - 1)
            event.accepted = true
        } else if (event.key === Qt.Key_Right) {
            root.selectedCategory = root.clampCategory(root.selectedCategory + 1)
            event.accepted = true
        }
    }

    Column {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 10

        // ---- Header: title + fullscreen toggle ----
        Item {
            width: parent.width
            height: 26

            Txt {
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                text: "Settings"
                color: Theme.fg
                font.family: Theme.fontName
                font.pixelSize: 16
                font.bold: true
            }

            IconButton {
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                size: 26
                glyphSize: 13
                glyph: root.fullscreen ? "\uf066" : "\uf065"
                fill: Theme.bg2
                radius: 8
                selected: root.fullscreen
                onClicked: SettingsManager.set("shell", "settingsFullscreen", !root.fullscreen)
            }
        }

        Row {
            width: parent.width
            height: parent.height - 26 - 10
            spacing: 14

            // ---- Category sidebar ----
            Column {
                width: 168
                height: parent.height
                spacing: 6

                Flickable {
                    id: categoryFlick
                    anchors.fill: parent
                    clip: true
                    contentHeight: categoryCol.height
                    boundsBehavior: Flickable.StopAtBounds

                    Column {
                        id: categoryCol
                        width: categoryFlick.width
                        spacing: 6

                        Repeater {
                            model: root.categories

                            delegate: Column {
                                required property int index
                                required property var modelData

                                width: categoryCol.width
                                spacing: 6

                                Rectangle {
                                    width: parent.width
                                    height: 40
                                    radius: 10
                                    color: root.selectedCategory === index ? Theme.acc : "transparent"

                                    Behavior on color {
                                        ColorAnimation { duration: 120 }
                                    }

                                    Row {
                                        anchors.fill: parent
                                        anchors.leftMargin: 12
                                        anchors.rightMargin: 12
                                        spacing: 10

                                        Txt {
                                            anchors.verticalCenter: parent.verticalCenter
                                            text: modelData.icon
                                            font.family: Theme.fontName
                                            font.pixelSize: 15
                                            color: root.selectedCategory === index ? Theme.sfg : Theme.fg
                                        }

                                        Txt {
                                            anchors.verticalCenter: parent.verticalCenter
                                            text: modelData.title
                                            color: root.selectedCategory === index ? Theme.sfg : Theme.fg
                                            font.family: Theme.fontName
                                            font.pixelSize: 13
                                            font.bold: root.selectedCategory === index
                                        }
                                    }

                                    MouseArea {
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: root.selectedCategory = index
                                    }
                                }
                            }
                        }
                    }
                }

                ScrollDragger {
                    target: categoryFlick
                    thumbWidth: 4
                    minThumbHeight: 26
                }
            }

            Rectangle {
                width: 1
                height: parent.height
                color: Theme.bg3
            }

            // ---- Content ----
            Item {
                width: parent.width - 168 - 14 - 1
                height: parent.height

                Loader {
                    id: contentLoader
                    anchors.fill: parent
                    sourceComponent: root.currentCategory && root.currentCategory.comp
                        ? root.categoryComp(root.currentCategory.comp) : root.aboutCategory
                }
            }
        }
    }
}
