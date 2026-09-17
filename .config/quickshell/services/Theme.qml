pragma Singleton
import QtQuick
import Quickshell
import QsIo 1.0
import "."
import "../components"

Singleton {
    id: theme

    // Fonts
    property string fontName: "Hack Nerd Font"
    readonly property string iconFont: "JetBrainsMono Nerd Font"
    readonly property string brandingFont: "opensuse"

    // Appearance mode: "dark" | "light". Read from $states2/m at startup
    // and refreshed by the `theme restore` IPC hook. Neutral colors follow it
    // while the accent stays as configured.
    property string mode: "dark"

    readonly property var dark: ({
        fg: "#e5e9f0", fg2: "#b4b4b4", fg3: "#b4b4b4",
        bg: "#2e3440", bg2: "#303642", bg3: "#303642",
        hover: "#1e2235", border: "#32364a",
        success: "#9ece6a", warning: "#ff9e64", danger: "#f7768e", info: "#7dcfff"
    })

    readonly property var light: ({
        fg: "#3b4252", fg2: "#4c566a", fg3: "#8791a1",
        bg: "#eceff4", bg2: "#e6e9ef", bg3: "#dde2ea",
        hover: "#d7dce4", border: "#c4cad6",
        success: "#4c8f62", warning: "#c97b2d", danger: "#d9534f", info: "#2b7db8"
    })

    // All colors come straight from $states2 (fg/fg2/fg3/bg/bg2/bg3/hover/
    // border/acc/sfg/success/warning/danger/info) — written by theme_body on
    // every apply from the master $states/themes palette. The dark/light maps
    // above are only the boot fallback for the brief moment before the first
    // apply writes the files.
    property string _fg: ""
    property string _fg2: ""
    property string _fg3: ""
    property string _bg: ""
    property string _bg2: ""
    property string _bg3: ""
    property string _hover: ""
    property string _border: ""
    property string _success: ""
    property string _warning: ""
    property string _danger: ""
    property string _info: ""

    readonly property color fg: _fg ? _fg : (mode === "light" ? light : dark).fg
    readonly property color fg2: _fg2 ? _fg2 : (mode === "light" ? light : dark).fg2
    readonly property color fg3: _fg3 ? _fg3 : (mode === "light" ? light : dark).fg3

    readonly property color bg: _bg ? _bg : (mode === "light" ? light : dark).bg
    readonly property color bg2: _bg2 ? _bg2 : (mode === "light" ? light : dark).bg2
    readonly property color bg3: _bg3 ? _bg3 : (mode === "light" ? light : dark).bg3
    readonly property color hover: _hover ? _hover : (mode === "light" ? light : dark).hover
    readonly property color border: _border ? _border : (mode === "light" ? light : dark).border

    // Accent fill and the foreground drawn on top of it (badges, selections,
    // toggles, scrollbars). Also read from $states2 on startup / IPC restore.
    property color acc: "#183048"
    property color sfg: "#e5e9f0"

    readonly property color success: _success ? _success : (mode === "light" ? light : dark).success
    readonly property color warning: _warning ? _warning : (mode === "light" ? light : dark).warning
    readonly property color danger: _danger ? _danger : (mode === "light" ? light : dark).danger
    readonly property color info: _info ? _info : (mode === "light" ? light : dark).info

    // Icon theme for app icons
    property string iconTheme: "Tela-nord-dark"
    readonly property string iconDir: Quickshell.env("HOME") + "/.local/share/icons/" + iconTheme

    // Reads the whole palette (mode + fg/fg2/fg3/bg/bg2/bg3/hover/border/
    // acc/sfg/success/warning/danger/info) straight from $states2 — the
    // dotfiles' single source of truth. Called on startup and re-fired by the
    // `theme restore` IPC hook after theme_body writes the files, so the
    // shell never needs to know how the palette was produced.
    function applyFromStates() {
        const st2 = SystemSettingsManager.states2
        const mode = String(QsIo.readFile(st2 + "/mode") || "").trim()
        if (mode === "d")
            theme.mode = "dark"
        else if (mode === "l")
            theme.mode = "light"
        const read = file => String(QsIo.readFile(st2 + "/" + file) || "").trim()
        const fg = read("fg");      if (fg)      theme._fg = fg
        const fg2 = read("fg2");    if (fg2)     theme._fg2 = fg2
        const fg3 = read("fg3");    if (fg3)     theme._fg3 = fg3
        const bg = read("bg");      if (bg)      theme._bg = bg
        const bg2 = read("bg2");    if (bg2)     theme._bg2 = bg2
        const bg3 = read("bg3");    if (bg3)     theme._bg3 = bg3
        const hover = read("hover");    if (hover)  theme._hover = hover
        const border = read("border");  if (border) theme._border = border
        // Prefer the 8-digit accent (acc8 = #RRGGBBAA, alpha LAST) so the
        // shell accent can be transparent. Qt parses 8-digit hex as
        // #AARRGGBB (alpha FIRST), so swap the alpha pair to the front.
        // Falls back to the opaque 6-digit acc when acc8 is missing.
        const acc8 = read("acc8")
        if (acc8 && acc8.length === 9 && acc8[0] === "#")
            theme.acc = "#" + acc8.slice(7) + acc8.slice(1, 7)
        else {
            const acc = read("acc");    if (acc)     theme.acc = acc
        }
        const sfg = read("sfg");    if (sfg)     theme.sfg = sfg
        const success = read("success");  if (success) theme._success = success
        const warning = read("warning");  if (warning) theme._warning = warning
        const danger = read("danger");    if (danger)  theme._danger = danger
        const info = read("info");    if (info)     theme._info = info
    }

    // Called by SettingsManager with the `appearance` section of
    // $states/settings.json. Only the non-color bits are persisted there now —
    // colors always come from $states2.
    function applyConfig(a) {
        const c = a || {}
        if (c.font)
            fontName = c.font
        if (c.iconTheme)
            iconTheme = c.iconTheme
    }

    // Toggle runs the dotfiles' switch pipeline (theme_main switch): it flips
    // mode, regenerates the palette into $states2, and theme_body fires the
    // `theme restore` IPC hook so this shell re-reads the new colors.
    function toggleMode() {
        themeLauncher.launch(["theme_main", "switch"])
    }

    property DetachedLauncher themeLauncher: DetachedLauncher {}

    Component.onCompleted: theme.applyFromStates()
}
