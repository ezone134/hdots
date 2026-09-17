import QtQuick
import "."
import "../../components"
import "../../services"

// Window Manager category: live editor for the Hyprland configs shipped in
// $hypr_conf/configs/*.lua (general.lua / blur.lua / shadow.lua /
// opacity.lua / shader.lua). The configs are GENERATED artifacts — the
// source of truth is the channel-scoped state files in $states. Every
// change therefore: writes <key>_<channel> into $states, then fires the
// matching restore script (general_main restore / blur_main restore /
// shadow_main restore / opacity_main restore / shader_main restore) which
// regenerates the config file from the state files. $states2/s picks
// the active channel (n/d/l). Values reload when the channel changes, so
// the section always shows the channel currently being edited.
Item {
    id: root

    readonly property string ch: SystemSettingsManager.channel

    // ---- General (general_main restore) ----
    property real gapsIn: 7
    property real gapsOut: 10
    property real rounding: 10
    property real roundingPower: 2
    property real borderSize: 3
    property bool resizeOnBorder: true
    property bool allowTearing: true
    property int layoutIndex: 0

    // ---- Blur (blur_main restore) ----
    property bool blurOn: true
    property real blurSize: 2
    property real blurPasses: 4
    property real blurVibrancy: 0.1

    // ---- Shadow (shadow_main restore) ----
    property bool shadowOn: true
    property real shadowRange: 8
    property real shadowRenderPower: 1
    property int shadowColorIndex: 0

    // ---- Opacity (opacity_main restore) ----
    property real opacityActive: 0.8
    property real opacityInactive: 0.5

    // ---- Shader (shader_main) ----
    // saturations/N.glsl : N=0 gray .. 10 normal .. 20 = 2.0× vibrance
    property real shaderVibrance: 1.0
    property bool shaderOn: false
    property bool _shaderBusy: false

    readonly property var layoutValues: ["dwindle", "master"]
    readonly property var layoutEntries: ["Dwindle", "Master"]

    readonly property var shadowColors: [
        { label: "Just Black", value: "0xee1a1a1a" },
        { label: "Deep Black", value: "0xee000000" },
        { label: "Green", value: "0xaa4caf50" },
        { label: "Blue", value: "0xaa42a5f5" },
        { label: "Cyan", value: "0xaa00bcd4" },
        { label: "Purple", value: "0xaa9c27b0" },
        { label: "Rose", value: "0xaae91e63" },
        { label: "Orange", value: "0xaaff9800" }
    ]

    function sf(file) {
        return SystemSettingsManager.states + "/" + file + "_" + root.ch
    }

    // Write one channel-scoped state file, then fire the restore script that
    // regenerates the matching configs/*.lua from the state files.
    function commit(file, value, script, arg) {
        SystemSettingsManager.writeFiles([{ path: root.sf(file), value: String(value) }], () => {
            SystemSettingsManager.apply(script, arg || "")
        })
    }

    function shaderPathForVibrance(v) {
        const n = Math.max(0, Math.min(20, Math.round(v * 10)))
        return SystemSettingsManager.shaders + "/saturations/" + n + ".glsl"
    }

    function vibranceFromShaderPath(p) {
        const base = String(p || "").trim().split("/").pop() || ""
        const num = parseInt(base.replace(".glsl", ""), 10)
        if (isNaN(num))
            return 1.0
        return Math.max(0, Math.min(2, num / 10))
    }

    function commitShaderVibrance(v) {
        if (root._shaderBusy)
            return
        root._shaderBusy = true
        root.shaderVibrance = Math.max(0, Math.min(2, v))
        const n = Math.max(0, Math.min(20, Math.round(root.shaderVibrance * 10)))
        SystemSettingsManager.apply(SystemSettingsManager.qsScripts + "/shader_main", "set " + n)
        root.shaderOn = true
        root._shaderBusy = false
    }

    function commitShaderOff() {
        if (root._shaderBusy)
            return
        root._shaderBusy = true
        SystemSettingsManager.apply(SystemSettingsManager.qsScripts + "/shader_main", "off")
        root.shaderOn = false
        root._shaderBusy = false
    }

    function reload() {
        SystemSettingsManager.readBatch([
            root.sf("gaps_in"), root.sf("gaps_out"), root.sf("rounding"),
            root.sf("rounding_power"), root.sf("border_size"),
            root.sf("resize_on_border"), root.sf("allow_tearing"), root.sf("layout"),
            root.sf("hypr_blur"), root.sf("hypr_blur_size"), root.sf("hypr_blur_passes"),
            root.sf("hypr_blur_vibrancy"),
            root.sf("shadow_enabled"), root.sf("shadow_range"),
            root.sf("shadow_render_power"), root.sf("shadow_rgba"),
            root.sf("opacity_active"), root.sf("opacity_inactive"),
            root.sf("shader_state"), root.sf("shader_val")
        ], r => {
            let i = 0
            root.gapsIn = parseFloat(r[i++] || "7")
            root.gapsOut = parseFloat(r[i++] || "10")
            root.rounding = parseFloat(r[i++] || "10")
            root.roundingPower = parseFloat(r[i++] || "2")
            root.borderSize = parseFloat(r[i++] || "3")
            root.resizeOnBorder = (r[i++] || "1").trim() === "1"
            root.allowTearing = (r[i++] || "1").trim() === "1"
            const layout = (r[i++] || "dwindle").trim()
            root.layoutIndex = Math.max(0, root.layoutValues.indexOf(layout))

            root.blurOn = (r[i++] || "1").trim() === "1"
            root.blurSize = parseFloat(r[i++] || "2")
            root.blurPasses = parseFloat(r[i++] || "4")
            root.blurVibrancy = parseFloat(r[i++] || "0.1")

            root.shadowOn = (r[i++] || "1").trim() === "1"
            root.shadowRange = parseFloat(r[i++] || "8")
            root.shadowRenderPower = parseFloat(r[i++] || "1")
            const rgba = (r[i++] || "0xee1a1a1a").trim()
            let ci = 0
            for (let k = 0; k < root.shadowColors.length; k++)
                if (root.shadowColors[k].value === rgba)
                    ci = k
            root.shadowColorIndex = ci

            root.opacityActive = parseFloat(r[i++] || "0.8")
            root.opacityInactive = parseFloat(r[i++] || "0.5")

            const shaderState = (r[i++] || "0").trim()
            const shaderVal = (r[i++] || "").trim()
            root.shaderOn = shaderState === "1"
            if (shaderVal.length > 0)
                root.shaderVibrance = root.vibranceFromShaderPath(shaderVal)
            else if (!root.shaderOn)
                root.shaderVibrance = 1.0
        })
    }

    Component.onCompleted: root.reload()

    Connections {
        target: SystemSettingsManager
        function onChannelChanged() { root.reload() }
        // After any apply() completes (e.g. a *_main reset) re-read the state
        // files so the section reflects the values the scripts wrote.
        function onApplied() { root.reload() }
    }

    // ---- UI ----
    Flickable {
        id: wmFlick
        anchors.fill: parent
        clip: true
        contentHeight: column.height
        boundsBehavior: Flickable.StopAtBounds

        Column {
            id: column
            width: wmFlick.width
            spacing: 4

            // ================= General =================
            WmHeader { label: "GENERAL  ·  general_main restore" }

            WmSlider { label: "Inner Gaps"; min: 1; max: 10; step: 1; value: root.gapsIn
                onChanged: v => { root.gapsIn = v; root.commit("gaps_in", v, "general_main", "restore") } }
            WmSlider { label: "Outer Gaps"; min: 1; max: 10; step: 1; value: root.gapsOut
                onChanged: v => { root.gapsOut = v; root.commit("gaps_out", v, "general_main", "restore") } }
            WmSlider { label: "Window Rounding"; min: 1; max: 16; step: 1; value: root.rounding
                onChanged: v => { root.rounding = v; root.commit("rounding", v, "general_main", "restore") } }
            WmSlider { label: "Rounding Power"; min: 1; max: 10; step: 1; value: root.roundingPower
                onChanged: v => { root.roundingPower = v; root.commit("rounding_power", v, "general_main", "restore") } }
            WmSlider { label: "Border Size"; min: 1; max: 10; step: 1; value: root.borderSize
                onChanged: v => { root.borderSize = v; root.commit("border_size", v, "general_main", "restore") } }
            WmToggle { label: "Resize on Border"
                hint: "Allow resizing windows by dragging their border"
                value: root.resizeOnBorder
                onChanged: v => { root.resizeOnBorder = v; root.commit("resize_on_border", v ? "1" : "0", "general_main", "restore") } }
            WmToggle { label: "Allow Tearing"
                hint: "Enable tearing for fullscreen windows (see Hyprland wiki)"
                value: root.allowTearing
                onChanged: v => { root.allowTearing = v; root.commit("allow_tearing", v ? "1" : "0", "general_main", "restore") } }
            WmCombo { label: "Layout"; entries: root.layoutEntries; currentIndex: root.layoutIndex
                onChanged: i => {
                    root.layoutIndex = i
                    root.commit("layout", root.layoutValues[i], "general_main", "restore")
                } }

            SettingsAction {
                width: parent.width
                label: "Restore Defaults"
                glyph: "\uf0e2"
                hint: "Clears general.lua — Hyprland default applies"
                onClicked: SystemSettingsManager.apply("general_main", "reset")
            }

            Item { width: 1; height: 8 }

            // ================= Blur =================
            // Mirrors scripts/blur_main → helpers/blur_body:
            // toggle|on|off|restore|reset + channel state hypr_blur_*
            // (blur notify filter lives in Settings → Notifications)
            WmHeader { label: "BLUR  ·  blur_main" }

            WmToggle { label: "Blur"
                hint: "blur_main on/off — decoration.blur.enabled"
                value: root.blurOn
                onChanged: v => {
                    root.blurOn = v
                    // on/off write hypr_blur_<ch> and regenerate blur.lua (same as CLI)
                    SystemSettingsManager.apply("blur_main", v ? "on" : "off")
                } }
            WmSlider { label: "Blur Size"; min: 1; max: 10; step: 1; value: root.blurSize
                onChanged: v => { root.blurSize = v; root.commit("hypr_blur_size", v, "blur_main", "restore") } }
            WmSlider { label: "Blur Passes"; min: 1; max: 10; step: 1; value: root.blurPasses
                onChanged: v => { root.blurPasses = v; root.commit("hypr_blur_passes", v, "blur_main", "restore") } }
            WmSlider { label: "Blur Vibrancy"; min: 0; max: 1; step: 0.1; value: root.blurVibrancy
                onChanged: v => { root.blurVibrancy = v; root.commit("hypr_blur_vibrancy", v, "blur_main", "restore") } }

            SettingsAction {
                width: parent.width
                label: "Reload / Restore"
                glyph: "\uf021"
                hint: "blur_main restore — rewrite blur.lua from state"
                onClicked: SystemSettingsManager.apply("blur_main", "restore")
            }
            SettingsAction {
                width: parent.width
                label: "Reset to Hyprland defaults"
                glyph: "\uf0e2"
                hint: "blur_main reset — empty blur.lua"
                onClicked: SystemSettingsManager.apply("blur_main", "reset")
            }

            Item { width: 1; height: 8 }

            // ================= Shadow =================
            WmHeader { label: "SHADOW  ·  shadow_main restore" }

            WmToggle { label: "Shadows"
                hint: "Draw drop shadows under windows"
                value: root.shadowOn
                onChanged: v => { root.shadowOn = v; root.commit("shadow_enabled", v ? "1" : "0", "shadow_main", "restore") } }
            WmSlider { label: "Shadow Range"; min: 1; max: 100; step: 1; value: root.shadowRange
                onChanged: v => { root.shadowRange = v; root.commit("shadow_range", v, "shadow_main", "restore") } }
            WmSlider { label: "Render Power"; min: 1; max: 10; step: 1; value: root.shadowRenderPower
                onChanged: v => { root.shadowRenderPower = v; root.commit("shadow_render_power", v, "shadow_main", "restore") } }
            WmCombo { label: "Shadow Color"
                entries: { const e = []; for (let k = 0; k < root.shadowColors.length; k++) e.push(root.shadowColors[k].label); return e }
                currentIndex: root.shadowColorIndex
                onChanged: i => {
                    root.shadowColorIndex = i
                    root.commit("shadow_rgba", root.shadowColors[i].value, "shadow_main", "restore")
                } }

            SettingsAction {
                width: parent.width
                label: "Restore Defaults"
                glyph: "\uf0e2"
                hint: "Clears shadow.lua — Hyprland default applies"
                onClicked: SystemSettingsManager.apply("shadow_main", "reset")
            }

            Item { width: 1; height: 8 }

            // ================= Opacity =================
            WmHeader { label: "OPACITY  ·  opacity_main restore" }

            WmSlider { label: "Active Opacity"; min: 0.1; max: 1; step: 0.1; value: root.opacityActive
                hint: "1.0 disables transparency on the focused window"
                onChanged: v => { root.opacityActive = v; root.commit("opacity_active", v, "opacity_main", "restore") } }
            WmSlider { label: "Inactive Opacity"; min: 0.1; max: 1; step: 0.1; value: root.opacityInactive
                onChanged: v => { root.opacityInactive = v; root.commit("opacity_inactive", v, "opacity_main", "restore") } }

            SettingsAction {
                width: parent.width
                label: "Restore Defaults"
                glyph: "\uf0e2"
                hint: "Clears opacity.lua — Hyprland default applies"
                onClicked: SystemSettingsManager.apply("opacity_main", "reset")
            }

            Item { width: 1; height: 8 }

            // ================= Shader =================
            // 0.glsl gray · 10.glsl normal (1.0) · 20.glsl 200% vibrance
            WmHeader { label: "SCREEN SHADER  ·  shader_main" }

            WmToggle { label: "Screen Shader"
                hint: "shader_main on/off — decoration.screen_shader"
                value: root.shaderOn
                onChanged: v => {
                    if (v)
                        root.commitShaderVibrance(root.shaderVibrance)
                    else
                        root.commitShaderOff()
                } }
            WmSlider { label: "Vibrance / Saturation"; min: 0; max: 2; step: 0.1; value: root.shaderVibrance
                hint: "0 gray · 1.0 normal · 2.0 = 200% (N.glsl, N=v×10)"
                onChanged: v => root.commitShaderVibrance(v) }

            SettingsAction {
                width: parent.width
                label: "Reload / Restore"
                glyph: "\uf021"
                hint: "shader_main restore — apply shader_val if enabled"
                onClicked: SystemSettingsManager.apply(SystemSettingsManager.qsScripts + "/shader_main", "restore")
            }
            SettingsAction {
                width: parent.width
                label: "Reset to Hyprland defaults"
                glyph: "\uf0e2"
                hint: "shader_main reset — empty shader.lua"
                onClicked: SystemSettingsManager.apply(SystemSettingsManager.qsScripts + "/shader_main", "reset")
            }

            Item { width: 1; height: 14 }
        }
    }

    ScrollDragger {
        target: wmFlick
        thumbWidth: 5
        minThumbHeight: 30
    }
}
