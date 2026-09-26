import QtQuick
import Quickshell.Wayland
import Quickshell
import "."
import "../services"

Scope {
    id: root

    property bool barVisible: true
    property string mode: StateController.osdMode
    // Settings fullscreen: the settings panel covers the whole screen when
    // open. Persisted as shell.settingsFullscreen; drives the window size,
    // the exclusive zone and the morph surface flag.
    readonly property bool settingsFullscreen:
        root.mode === "settings" && SettingsManager.config.shell.settingsFullscreen === true
    // True while the cursor is anywhere over the bar window in
    // collapsed-by-default mode; drives the pill's expand-on-hover so the
    // island expands over the whole rect, not just the clock text.
    property bool barHovered: false

    Variants {
        model: Quickshell.screens

        PanelWindow {
            id: panelWindow
            required property var modelData
            screen: modelData
            visible: true
            color: "transparent"
            focusable: true

            // Only the island itself accepts input. The window spans the
            // full top width (for centering + exclusive zone), but every
            // transparent pixel outside the MorphSurface passes clicks
            // through to the windows below instead of eating them.
            mask: Region {
                item: morphSurface
            }

            anchors {
                top: true
                left: true
                right: true
            }

            implicitHeight: root.settingsFullscreen ? modelData.height : morphSurface.implicitHeight + 40
            // Fullscreen panel reserves nothing (it covers the screen); the
            // floating bar keeps its exclusive zone as usual.
            exclusiveZone: root.settingsFullscreen ? 0 : (SettingsManager.config.shell.floating === false ? 40 : 65)
            WlrLayershell.layer: WlrLayer.Top
            // Exclusive focus for modal panels only — driven by
            // PanelRegistry.exclusiveModes so adding a panel can't forget it.
            WlrLayershell.keyboardFocus:
                PanelRegistry.exclusiveModes.indexOf(mode) !== -1
                ? WlrKeyboardFocus.Exclusive
                : WlrKeyboardFocus.None

            Item {
                id: morphContainer
                // Full-width wrapper so the MorphSurface stays horizontally
                // centered across the screen; the surface itself always
                // shrink-wraps the pill/panel content.
                anchors {
                    top: parent.top
                    left: parent.left
                    right: parent.right
                }
                focus: true

                Keys.onPressed: function(event) {
                    if (StateController.osdMode === "workspace") {
                        event.accepted = false
                        return
                    }

                    // The polkit dialog dismisses itself via cancel; don't
                    // yank it away while an auth flow is still active.
                    if (event.key === Qt.Key_Escape && StateController.osdMode !== "pill" && StateController.osdMode !== "polkit") {
                        StateController.osdMode = "pill"
                        event.accepted = true
                    }
                }

                MorphSurface {
                    id: morphSurface
                    anchors {
                        top: parent.top
                        horizontalCenter: parent.horizontalCenter
                    }
                    fullscreen: root.settingsFullscreen
                    pillComponent: Component {
                        Pill {
                            externalHover: root.barHovered
                        }
                    }
                }

                // Hover + right-click on the pill surface. Hover expand only
                // when enabled; right-click always opens Settings.
                MouseArea {
                    id: pillHover
                    anchors.fill: morphSurface
                    z: 1000
                    visible: StateController.osdMode === "pill"
                    hoverEnabled: SettingsManager.config.shell.expandOnHover !== false
                    acceptedButtons: Qt.RightButton
                    cursorShape: Qt.PointingHandCursor
                    onEntered: {
                        if (SettingsManager.config.shell.expandOnHover !== false)
                            root.barHovered = true
                    }
                    onExited: root.barHovered = false
                    onClicked: function(mouse) {
                        // Compact island only (not while hover-expanded).
                        if (mouse.button === Qt.RightButton && !root.barHovered) {
                            mouse.accepted = true
                            StateController.settings()
                        }
                    }
                }
            }

            ShellIpc {}
        }
    }
}