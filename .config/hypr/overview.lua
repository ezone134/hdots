

-- .config/hypr/hyprland.lua
hl.config({
    plugin = {
        scrolloverview = {
            gesture_distance = 300, -- how far is the "max" for the gesture
--            scale = 0.5,            -- preferred overview scale
            scale = 0.35,            -- preferred overview scale
            workspace_gap = 0,
--            layout = "vertical",    -- vertical or horizontal
            layout = "vertical",    -- vertical or horizontal
            wallpaper = 0,          -- 0: global only, 1: per-workspace only, 2: both
--            blur = false,           -- blur only the main overview wallpaper
            blur = true,           -- blur only the main overview wallpaper

            shadow = {
--                enabled = false,
                enabled = 0,
                range = 50,
                render_power = 3,
                color = 0xee1a1a1a,
            },
        },
    },
})


-- hyprland.lua
hl.bind("SUPER + Tab", function()
    hl.plugin.scrolloverview.overview("toggle")
--    hl.plugin.scrolloverview.overview("on")
end)

hl.bind("SUPER + ESCAPE", function()
--    hl.plugin.scrolloverview.overview("toggle")
    hl.plugin.scrolloverview.overview("off")
end)
