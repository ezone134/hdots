pragma ComponentBehavior: Bound
import QtQuick
import "."
import "../../components"
import "../../services"

// Month calendar with ‹ › navigation (Left/Right keys too), today
// highlighted, and the viewed month's special occasions listed on the right.
// Data comes from CalendarManager (~/.config/hdots/calendar_events.json, live-watched).
Item {
    id: root

    property date today: new Date()
    property int viewYear: today.getFullYear()
    property int viewMonth: today.getMonth()   // 0-11

    property var cells: []
    property var occasions: []

    readonly property int cellSize: 34
    readonly property int gridWidth: 7 * root.cellSize + 6 * 2
    readonly property int sideWidth: 185

    implicitWidth: 500
    implicitHeight: 320

    focus: true
    Keys.priority: Keys.BeforeItem

    function pad2(n) {
        return n < 10 ? "0" + n : String(n)
    }

    function daysInMonth(y, m) {
        return new Date(y, m + 1, 0).getDate()
    }

    // 0 = Sunday
    function firstWeekday(y, m) {
        return new Date(y, m, 1).getDay()
    }

    function monthLabel() {
        return Qt.locale().monthName(root.viewMonth) + " " + root.viewYear
    }

    function prevMonth() {
        if (root.viewMonth === 0) {
            root.viewMonth = 11
            root.viewYear--
        } else {
            root.viewMonth--
        }
    }

    function nextMonth() {
        if (root.viewMonth === 11) {
            root.viewMonth = 0
            root.viewYear++
        } else {
            root.viewMonth++
        }
    }

    function isTodayOccasion(day) {
        const t = root.today
        return root.viewYear === t.getFullYear() && root.viewMonth === t.getMonth() && day === t.getDate()
    }

    function refresh() {
        root.today = new Date()
        const first = root.firstWeekday(root.viewYear, root.viewMonth)
        const days = root.daysInMonth(root.viewYear, root.viewMonth)
        const t = root.today
        const out = []
        for (let i = 0; i < 42; i++) {
            const day = i - first + 1
            out.push({
                day: day,
                inMonth: day >= 1 && day <= days,
                isToday: day === t.getDate() && root.viewMonth === t.getMonth() && root.viewYear === t.getFullYear()
            })
        }
        root.cells = out
        root.occasions = CalendarManager.eventsForMonth(root.viewYear, root.viewMonth)
    }

    onViewMonthChanged: refresh()
    onViewYearChanged: refresh()

    Connections {
        target: CalendarManager
        function onOccasionsChanged() {
            root.refresh()
        }
    }

    Keys.onPressed: event => {
        if (event.key === Qt.Key_Escape) {
            StateController.returnToPill()
            event.accepted = true
        } else if (event.key === Qt.Key_Left) {
            root.prevMonth()
            event.accepted = true
        } else if (event.key === Qt.Key_Right) {
            root.nextMonth()
            event.accepted = true
        } else if (event.key === Qt.Key_Up) {
            root.viewYear--
            event.accepted = true
        } else if (event.key === Qt.Key_Down) {
            root.viewYear++
            event.accepted = true
        } else {
            event.accepted = true
        }
    }

    Row {
        id: mainRow
        anchors.fill: parent
        anchors.margins: 16
        spacing: 16

        Column {
            id: calCol
            width: root.gridWidth
            spacing: 8

            // Month header with prev/next arrows
            Item {
                width: parent.width
                height: 28

                Rectangle {
                    id: prevBtn
                    width: 24
                    height: 24
                    radius: 12
                    color: prevHover.containsMouse ? Theme.hover : "transparent"
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter

                    Txt {
                        anchors.centerIn: parent
                        text: "\uf053"
                        font.family: Theme.fontName
                        font.pixelSize: 12
                        color: Theme.fg
                    }

                    MouseArea {
                        id: prevHover
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.prevMonth()
                    }
                }

                Txt {
                    anchors.centerIn: parent
                    text: root.monthLabel()
                    font.family: Theme.fontName
                    font.pixelSize: 14
                    font.bold: true
                    color: Theme.fg
                }

                Rectangle {
                    id: nextBtn
                    width: 24
                    height: 24
                    radius: 12
                    color: nextHover.containsMouse ? Theme.hover : "transparent"
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter

                    Txt {
                        anchors.centerIn: parent
                        text: "\uf054"
                        font.family: Theme.fontName
                        font.pixelSize: 12
                        color: Theme.fg
                    }

                    MouseArea {
                        id: nextHover
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.nextMonth()
                    }
                }
            }

            // Weekday labels
            Row {
                spacing: 2
                Repeater {
                    model: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
                    delegate: Txt {
                        required property string modelData
                        width: root.cellSize
                        horizontalAlignment: Text.AlignHCenter
                        text: modelData
                        font.family: Theme.fontName
                        font.pixelSize: 11
                        color: Theme.fg3
                    }
                }
            }

            // Day grid (42 cells, Sunday-first)
            Grid {
                columns: 7
                spacing: 2
                Repeater {
                    model: root.cells
                    delegate: Rectangle {
                        required property var modelData
                        width: root.cellSize
                        height: root.cellSize
                        radius: 8
                        color: modelData.isToday ? Theme.acc
                              : (dayHover.containsMouse && modelData.inMonth ? Theme.hover : "transparent")

                        Behavior on color {
                            NumberAnimation { duration: 120 }
                        }

                        Txt {
                            anchors.centerIn: parent
                            text: modelData.inMonth ? String(modelData.day) : ""
                            font.family: Theme.fontName
                            font.pixelSize: 13
                            font.bold: modelData.isToday
                            color: modelData.isToday ? Theme.sfg : Theme.fg
                        }

                        MouseArea {
                            id: dayHover
                            anchors.fill: parent
                            hoverEnabled: true
                        }
                    }
                }
            }
        }

        // Divider
        Rectangle {
            width: 1
            color: Theme.border
            anchors.top: parent.top
            anchors.bottom: parent.bottom
        }

        // Special occasions for the viewed month
        Column {
            id: sideCol
            width: root.sideWidth
            spacing: 10

            Row {
                spacing: 6
                Txt {
                    text: "\uf073"
                    font.family: Theme.fontName
                    font.pixelSize: 13
                    color: Theme.fg2
                }
                Txt {
                    text: "Occasions"
                    font.family: Theme.fontName
                    font.pixelSize: 12
                    font.bold: true
                    color: Theme.fg2
                }
            }

            Item {
                width: parent.width
                height: 230
                clip: true

                Column {
                    width: parent.width
                    spacing: 6

                    Repeater {
                        model: root.occasions
                        delegate: Row {
                            required property var modelData
                            width: parent.width
                            spacing: 6

                            Txt {
                                width: 22
                                text: root.pad2(modelData.day)
                                font.family: Theme.fontName
                                font.pixelSize: 12
                                font.bold: true
                                color: root.isTodayOccasion(modelData.day) ? Theme.info : Theme.fg2
                            }

                            Txt {
                                width: parent.width - 28
                                elide: Text.ElideRight
                                text: modelData.names.join(" · ")
                                font.family: Theme.fontName
                                font.pixelSize: 12
                                color: root.isTodayOccasion(modelData.day) ? Theme.info : Theme.fg
                            }
                        }
                    }

                    Txt {
                        visible: root.occasions.length === 0
                        width: parent.width
                        text: "No special days this month"
                        font.family: Theme.fontName
                        font.pixelSize: 12
                        color: Theme.fg3
                        wrapMode: Text.WordWrap
                    }
                }
            }
        }
    }

    Component.onCompleted: {
        root.refresh()
        forceActiveFocus()
    }
}