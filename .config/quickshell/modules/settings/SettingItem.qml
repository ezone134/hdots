import QtQuick
import "."

// Delegate for one row of a SettingSection. Picks the widget component for
// the row's `type` (header/toggle/slider/combo/button) and hands it the
// row QObject + the engine. Rows are stable QObjects from SettingEngine,
// so widget bindings update live when the engine refreshes values.
Item {
    id: root

    property var row: null
    property var engine: null
    signal comboOpened()

    width: parent ? parent.width : 0
    implicitHeight: loader.implicitHeight

    readonly property Component _component: root.row ? (root.row.type === "header" ? headerCmp
        : root.row.type === "toggle" ? toggleCmp
        : root.row.type === "slider" ? sliderCmp
        : root.row.type === "combo" ? comboCmp
        : root.row.type === "button" ? buttonCmp
        : null) : null

    Loader {
        id: loader
        anchors.left: parent.left
        anchors.right: parent.right
        sourceComponent: root._component

        onLoaded: {
            loader.item.row = root.row
            loader.item.engine = root.engine
            if (loader.item.comboOpened)
                loader.item.comboOpened.connect(root.comboOpened)
        }
    }

    Component { id: headerCmp; SettingHeader {} }
    Component { id: toggleCmp; SettingToggle {} }
    Component { id: sliderCmp; SettingSlider {} }
    Component { id: comboCmp; SettingCombo {} }
    Component { id: buttonCmp; SettingButton {} }
}
