import QtQuick
import "../../components"
import "../../services"

// Capture category: screenshot + screen recording config, live test buttons.
// Persisted under $states/settings.json "capture" via SettingsManager.set().
// Captures themselves run through CaptureManager (grim/slurp/wf-recorder).
Column {
    id: root
    spacing: 14

    function fpsToValue(f) { return Math.max(0, Math.min(1, (f - 15) / 105)) }
    function valueToFps(v) { return Math.round(15 + v * 105) }

    Txt {
        text: "Capture"
        color: Theme.fg
        font.family: Theme.fontName
        font.pixelSize: 13
        font.bold: true
    }

    Txt {
        width: 440
        wrapMode: Text.WordWrap
        text: "Screenshots (grim) and screen recording (wf-recorder). Bind keybinds to `quickshell ipc call capture shot_full|shot_region|shot_window|record_toggle` — or use the test buttons below."
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 11
    }

    Txt {
        text: "Screenshots"
        color: Theme.fg2
        font.family: Theme.fontName
        font.pixelSize: 11
        font.bold: true
    }

    Txt {
        text: "Folder"
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 11
    }

    Rectangle {
        width: 440
        height: 38
        radius: 10
        color: Theme.bg2
        border.width: 1
        border.color: shotDirInput.activeFocus ? Theme.acc : Theme.bg3

        TxtInput {
            id: shotDirInput
            anchors.fill: parent
            anchors.leftMargin: 12
            anchors.rightMargin: 12
            verticalAlignment: TextInput.AlignVCenter
            color: Theme.fg
            font.family: Theme.fontName
            font.pixelSize: 13
            text: SettingsManager.config.capture.screenshotDir || ""
            selectByMouse: true

            onEditingFinished: {
                SettingsManager.set("capture", "screenshotDir", shotDirInput.text.trim())
            }
        }
    }

    Txt {
        width: 440
        wrapMode: Text.WordWrap
        text: "Leave empty for ~/Pictures/Screenshots. A leading ~ is expanded."
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 11
    }

    Txt {
        text: "Format"
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 11
    }

    Row {
        spacing: 8

        Repeater {
            model: ["png", "jpg"]

            Rectangle {
                required property int index
                required property string modelData
                width: 72
                height: 30
                radius: 8
                color: SettingsManager.config.capture.format === modelData ? Theme.acc : Theme.bg2
                border.width: 1
                border.color: Theme.bg3

                Behavior on color {
                    ColorAnimation { duration: 120 }
                }

                Txt {
                    anchors.centerIn: parent
                    text: modelData.toUpperCase()
                    color: SettingsManager.config.capture.format === modelData ? Theme.sfg : Theme.fg
                    font.family: Theme.fontName
                    font.pixelSize: 12
                    font.bold: true
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: SettingsManager.set("capture", "format", modelData)
                }
            }
        }
    }

    Item {
        width: 440
        height: 44

        Column {
            anchors.left: parent.left
            anchors.right: copySwitch.left
            anchors.rightMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2

            Txt {
                text: "Copy to clipboard"
                color: Theme.fg
                font.family: Theme.fontName
                font.pixelSize: 13
                font.bold: true
            }

            Txt {
                width: parent.width
                wrapMode: Text.WordWrap
                text: "After a screenshot, also copy the image with wl-copy."
                color: Theme.fg3
                font.family: Theme.fontName
                font.pixelSize: 11
            }
        }

        ToggleSwitch {
            id: copySwitch
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            checked: SettingsManager.config.capture.copyScreenshot === true
            onToggled: value => SettingsManager.set("capture", "copyScreenshot", value)
        }
    }

    Txt {
        text: "Test buttons"
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 11
    }

    Row {
        spacing: 8

        Repeater {
            model: [
                { label: "Full", action: "full" },
                { label: "Region", action: "region" },
                { label: "Window", action: "window" }
            ]

            Rectangle {
                required property int index
                required property var modelData
                width: 96
                height: 32
                radius: 9
                color: Theme.bg2
                border.width: 1
                border.color: Theme.bg3

                Txt {
                    anchors.centerIn: parent
                    text: modelData.label
                    color: Theme.fg
                    font.family: Theme.fontName
                    font.pixelSize: 12
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        if (modelData.action === "full")
                            CaptureManager.shotFull()
                        else if (modelData.action === "region")
                            CaptureManager.shotRegion()
                        else
                            CaptureManager.shotWindow()
                    }
                }
            }
        }
    }

    Txt {
        text: "Screen recording"
        color: Theme.fg2
        font.family: Theme.fontName
        font.pixelSize: 11
        font.bold: true
    }

    Txt {
        text: "Folder"
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 11
    }

    Rectangle {
        width: 440
        height: 38
        radius: 10
        color: Theme.bg2
        border.width: 1
        border.color: recDirInput.activeFocus ? Theme.acc : Theme.bg3

        TxtInput {
            id: recDirInput
            anchors.fill: parent
            anchors.leftMargin: 12
            anchors.rightMargin: 12
            verticalAlignment: TextInput.AlignVCenter
            color: Theme.fg
            font.family: Theme.fontName
            font.pixelSize: 13
            text: SettingsManager.config.capture.recordDir || ""
            selectByMouse: true

            onEditingFinished: {
                SettingsManager.set("capture", "recordDir", recDirInput.text.trim())
            }
        }
    }

    Txt {
        text: "Frame rate: " + (SettingsManager.config.capture.fps || 60) + " fps"
        color: Theme.fg3
        font.family: Theme.fontName
        font.pixelSize: 11
    }

    Item {
        id: fpsTrack
        width: 400
        height: 24

        Rectangle {
            anchors.fill: parent
            radius: height / 2
            color: Theme.bg2
        }

        Item {
            anchors.fill: parent
            anchors.margins: 3

            Rectangle {
                id: fpsFill
                height: parent.height
                width: Math.max(height, parent.width * root.fpsToValue(SettingsManager.config.capture.fps || 60))
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                radius: height / 2
                color: Theme.acc
            }
        }

        MouseArea {
            anchors.fill: parent
            onPressed: mouse => {
                SettingsManager.set("capture", "fps", root.valueToFps(fpsTrack.pctFromX(mouse.x)))
            }
            onPositionChanged: mouse => {
                if (pressed)
                    SettingsManager.set("capture", "fps", root.valueToFps(fpsTrack.pctFromX(mouse.x)))
            }
        }

        function pctFromX(x) {
            var p = (x - 3) / (width - 6)
            return Math.max(0, Math.min(1, p))
        }
    }

    Rectangle {
        width: 180
        height: 36
        radius: 10
        color: CaptureManager.recording ? Theme.danger : Theme.acc

        Behavior on color {
            ColorAnimation { duration: 150 }
        }

        Txt {
            anchors.centerIn: parent
            text: CaptureManager.recording ? "Stop recording" : "Start recording (region)"
            color: Theme.sfg
            font.family: Theme.fontName
            font.pixelSize: 12
            font.bold: true
        }

        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: CaptureManager.recordToggle()
        }
    }
}
