pragma ComponentBehavior: Bound
import QtQuick
import "."
import "../../components"
import "../../services"

// Weather island: current conditions (temp, condition, city, feels-like,
// humidity, wind) plus a 12-hour forecast strip. Refreshes on open so the
// data is fresh on demand; right-clicking the pill chip or the panel title
// forces a manual refresh. Esc closes, ←/→ (or Tab) cycles the hour strip.
Item {
    id: root
    implicitWidth: 720
    implicitHeight: 320
    focus: true

    Component.onCompleted: {
        forceActiveFocus()
        WeatherManager.refresh()
    }

    onVisibleChanged: {
        if (visible) {
            forceActiveFocus()
            WeatherManager.refresh()
        }
    }

    property int selectedHour: -1
    property real contentOpacity: 0
    property real contentScale: 0.98

    // Once hourly data lands, point the selection at the first hour.
    Connections {
        target: WeatherManager
        function onHourlyCountChanged() {
            if (root.selectedHour < 0 && WeatherManager.hourlyCount > 0)
                root.selectedHour = 0
        }
    }

    property string bigTemp: WeatherManager.ready
        ? WeatherManager.temperature + "\u00b0"
        : "--"
    property string bigGlyph: WeatherManager.glyph.length > 0
        ? WeatherManager.glyph
        : (WeatherManager.error.length > 0 ? "\uf071" : "\uf0c2")
    property string statusLine: WeatherManager.error.length > 0
        ? WeatherManager.error
        : (WeatherManager.loading ? "Fetching weather\u2026" : WeatherManager.condition)

    function hourSelected(i) {
        return root.selectedHour >= 0 && i === root.selectedHour
    }

    function selectedDetail() {
        if (root.selectedHour < 0 || root.selectedHour >= WeatherManager.hourlyCount)
            return ""
        const h = WeatherManager.hourly[root.selectedHour]
        return h.time + " \u00b7 " + h.temp + "\u00b0 " + h.label
    }

    Keys.onPressed: function(event) {
        if (event.key === Qt.Key_Escape) {
            StateController.pillReset()
            event.accepted = true
            return
        }
        if (event.key === Qt.Key_Left || event.key === Qt.Key_Up || (event.key === Qt.Key_Tab && (event.modifiers & Qt.ShiftModifier))) {
            if (root.selectedHour <= 0)
                root.selectedHour = WeatherManager.hourlyCount - 1
            else
                root.selectedHour = root.selectedHour - 1
            event.accepted = true
            return
        }
        if (event.key === Qt.Key_Right || event.key === Qt.Key_Down || (event.key === Qt.Key_Tab && !(event.modifiers & Qt.ShiftModifier))) {
            if (root.selectedHour >= WeatherManager.hourlyCount - 1)
                root.selectedHour = 0
            else
                root.selectedHour = root.selectedHour + 1
            event.accepted = true
        }
    }

    component DetailRow: Row {
        required property string icon
        required property string label
        required property string value
        spacing: 8

        Txt {
            width: 18
            text: parent.icon
            font.family: Theme.iconFont
            font.pixelSize: 14
            color: Theme.fg3
            anchors.verticalCenter: parent.verticalCenter
        }
        Txt {
            text: parent.label
            font.family: Theme.fontName
            font.pixelSize: 13
            color: Theme.fg3
            anchors.verticalCenter: parent.verticalCenter
        }
        Txt {
            text: parent.value
            font.family: Theme.fontName
            font.pixelSize: 13
            font.bold: true
            color: Theme.fg
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    Timer {
        id: contentInTimer
        interval: 80
        repeat: false
        running: true
        onTriggered: {
            root.contentOpacity = 1
            root.contentScale = 1
        }
    }

    Item {
        anchors.fill: parent
        opacity: root.contentOpacity
        scale: root.contentScale

        Behavior on opacity {
            NumberAnimation { duration: 200; easing.type: Easing.OutCubic }
        }
        Behavior on scale {
            NumberAnimation { duration: 200; easing.type: Easing.OutCubic }
        }

        Column {
            anchors.fill: parent
            anchors.margins: 16
            spacing: 10

            SectionTitle {
                icon: root.bigGlyph
                title: "Weather"
                subtitle: {
                    const parts = []
                    if (WeatherManager.city.length > 0)
                        parts.push(WeatherManager.city)
                    if (WeatherManager.ready)
                        parts.push("updated " + WeatherManager.updatedAt)
                    return parts.join("  \u00b7  ")
                }

                MouseArea {
                    anchors.fill: parent
                    acceptedButtons: Qt.RightButton
                    cursorShape: Qt.PointingHandCursor
                    onClicked: WeatherManager.refresh()
                }
            }

            Item {
                width: parent.width
                height: 104

                Row {
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 20

                    Txt {
                        anchors.verticalCenter: parent.verticalCenter
                        text: root.bigGlyph
                        font.family: Theme.iconFont
                        font.pixelSize: 56
                        color: Theme.fg
                    }

                    Column {
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 3

                        Txt {
                            text: root.bigTemp
                            font.family: Theme.fontName
                            font.pixelSize: 44
                            font.bold: true
                            color: Theme.fg
                        }
                        Txt {
                            text: root.statusLine
                            font.family: Theme.fontName
                            font.pixelSize: 14
                            color: Theme.fg2
                        }
                        Txt {
                            visible: WeatherManager.error.length > 0
                            text: "Right-click the weather chip to retry"
                            font.family: Theme.fontName
                            font.pixelSize: 11
                            color: Theme.fg3
                        }
                    }
                }

                Column {
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 9

                    DetailRow {
                        icon: "\uf769"
                        label: "Feels like"
                        value: WeatherManager.ready ? WeatherManager.feelsLike + "\u00b0" : "\u2014"
                    }
                    DetailRow {
                        icon: "\uf043"
                        label: "Humidity"
                        value: WeatherManager.ready ? WeatherManager.humidity + "%" : "\u2014"
                    }
                    DetailRow {
                        icon: "\uf72e"
                        label: "Wind"
                        value: WeatherManager.ready ? WeatherManager.windSpeed + " km/h" : "\u2014"
                    }
                }
            }

            Item {
                width: parent.width
                height: 82

                Row {
                    anchors.fill: parent
                    spacing: 8

                    Repeater {
                        model: WeatherManager.hourlyCount

                        delegate: Item {
                            required property int index
                            width: (parent.width - (WeatherManager.hourlyCount - 1) * 8) / WeatherManager.hourlyCount
                            height: parent.height

                            Rectangle {
                                anchors.fill: parent
                                radius: 14
                                color: root.hourSelected(index) ? Qt.rgba(0, 0, 0, 0.10) : Qt.rgba(0, 0, 0, 0.02)
                                border.width: root.hourSelected(index) ? 2 : 1
                                border.color: root.hourSelected(index) ? Theme.acc : Qt.rgba(1, 1, 1, 0.35)

                                Behavior on color {
                                    ColorAnimation { duration: 150 }
                                }
                                Behavior on border.color {
                                    ColorAnimation { duration: 150 }
                                }

                                Column {
                                    anchors.centerIn: parent
                                    spacing: 6

                                    Txt {
                                        anchors.horizontalCenter: parent.horizontalCenter
                                        text: WeatherManager.hourly[index].time
                                        font.family: Theme.fontName
                                        font.pixelSize: 11
                                        color: Theme.fg3
                                    }
                                    Txt {
                                        anchors.horizontalCenter: parent.horizontalCenter
                                        text: WeatherManager.hourly[index].glyph
                                        font.family: Theme.iconFont
                                        font.pixelSize: 20
                                        color: Theme.fg
                                    }
                                    Txt {
                                        anchors.horizontalCenter: parent.horizontalCenter
                                        text: WeatherManager.hourly[index].temp + "\u00b0"
                                        font.family: Theme.fontName
                                        font.pixelSize: 13
                                        font.bold: true
                                        color: Theme.fg
                                    }
                                }
                            }

                            MouseArea {
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onEntered: root.selectedHour = index
                                onClicked: root.selectedHour = index
                            }
                        }
                    }
                }
            }

            Row {
                width: parent.width
                spacing: 12

                Txt {
                    id: selectedDetail
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.selectedDetail()
                    font.family: Theme.fontName
                    font.pixelSize: 12
                    color: Theme.acc
                }

                Item {
                    width: parent.width - selectedDetail.implicitWidth - detailHint.implicitWidth - hintRow.implicitWidth - 24
                    height: 1
                }

                Txt {
                    id: detailHint
                    anchors.verticalCenter: parent.verticalCenter
                    text: "\u2190 \u2192 select hour"
                    font.family: Theme.fontName
                    font.pixelSize: 12
                    color: Theme.fg3
                }

                Txt {
                    id: hintRow
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Esc close"
                    font.family: Theme.fontName
                    font.pixelSize: 12
                    color: Theme.fg3
                }
            }
        }
    }
}
