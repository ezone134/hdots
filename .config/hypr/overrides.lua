vfr = true
disable_hyprland_guiutils_check = true
disable_hyprland_qtutils_check = true

hl.layer_rule({
    name      = "clean-rofi-fade",
    match     = { namespace = "^rofi$" },
    animation = "fade almostLinear",
})

hl.layer_rule({
    name         = "blur-rofi",
    match        = { namespace = "^rofi$" },
    blur         = true,
    ignore_alpha = 0.5,
})

hl.layer_rule({
    name         = "blur-notifications",
    match        = { namespace = "^notifications$" },
    blur         = true,
    ignore_alpha = 0.5,
})

hl.config({
    misc = {
        disable_hyprland_guiutils_check = true,
    },
})

hl.layer_rule({
    name         = "blur-zen-shell",
    match        = { namespace = "^zen-shell$" },
    blur         = true,
    ignore_alpha = 0.5,
})

hl.window_rule({
    name   = "polkit-mate-focus",
    match  = {
        class = "^polkit-mate-authentication-agent-1$",
    },
    float  = true,
    center = true,
    pin    = true,
})

hl.window_rule({
    name   = "always-float-color-picker",
    match  = {
        class = "^gcolor3$",
    },
    float  = true,
    center = true,
    pin    = true,
})

hl.curve("rofiPopCurve", { type = "bezier", points = { { 0.34, 1.56 }, { 0.64, 1 } } })

hl.animation({
    leaf    = "layers",
    enabled = true,
    speed   = 5,
    bezier  = "rofiPopCurve",
    style   = "popin 80%"
})

hl.layer_rule({
    name         = "rofi-custom-pop",
    match        = { namespace = "^rofi$" },
    animation    = "popin 85%",
    blur         = true,
    ignore_alpha = 0.5,
})

hl.window_rule({
    name   = "ad-loading-overlay",
    match  = {
        class = "^python3$",
        title = "^ad$",
    },
    float  = true,
    pin    = true,
    center = true,
    size   = "100% 100%",
})

hl.window_rule({
    name   = "always-float-file-chooser",
    match  = {
        title = "^(Open Files|Open File|Save File|Choose a file|File Upload)$",
    },
    float  = true,
    center = true,
    pin    = true,
    size   = { "60%", "50%" },
})

hl.window_rule({
    name   = "alacritty-float-rule",
    match  = {
        class = "^floating-script$",
    },
    float  = true,
    center = true,
    pin    = true,
})

hl.config({
    general = {
        layout = "scrolling",
    },
    scrolling = {
        column_width = 1.0,
        fullscreen_on_one_column = true,
    },
})

hl.env("HYPRCURSOR_THEME", "Hyper-Ice")
hl.env("HYPRCURSOR_SIZE", "24")
hl.env("XCURSOR_THEME", "Hyper-Ice")
hl.env("XCURSOR_SIZE", "24")
