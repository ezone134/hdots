---------------------
---- KEYBINDINGS ----
---------------------
--terminal    = "kitty $HOME"
terminal      = "alacritty"
fileManager   = "thunar"
menu          = "rofi-toggle"

local mainMod = "SUPER" -- Sets "Windows" key as main modifier





-- Example binds, see https://wiki.hypr.land/Configuring/Basics/Binds/ for more
hl.config({
    binds = {
        allow_workspace_cycles = false,
        workspace_back_and_forth = false
    }
})









    hl.bind(mainMod .. " + left", hl.dsp.focus({ direction = "left" }), { locked = true, repeating = true })                  -- dots | SUPER + Left   →   Focus Left Window
    hl.bind(mainMod .. " + right", hl.dsp.focus({ direction = "right" }), { locked = true, repeating = true })                                               -- dots | SUPER + Right   →   Focus Right Window


    hl.bind(mainMod .. " + up", hl.dsp.focus({ workspace = "e-1" }), { locked = true, repeating = false })                  -- dots | SUPER + Up   →   Focus Previous Workspace
    hl.bind(mainMod .. " + down", hl.dsp.focus({ workspace = "e+1" }), { locked = true, repeating = false })                                               -- dots | SUPER + Down   →   Focus Next Workspace


hl.bind(mainMod .. " + Return", hl.dsp.exec_cmd(terminal))                -- dots | SUPER + Enter   →   Open Terminal
local closeWindowBind = hl.bind(mainMod .. " + Q", hl.dsp.window.close()) -- dots | SUPER + Q   →   Close focused window
hl.bind(mainMod .. " + SHIFT + F",
    hl.dsp.window.fullscreen({                                            -- dots | SUPER + SHIFT + F   →   Toggle Fullscreen
        mode = "fullscreen",
        action = "toggle",
    }))

hl.bind(mainMod .. " + F", hl.dsp.window.float({ action = "toggle" })) -- dots | SUPER + F   →   Toggle Floating Mode


hl.bind(mainMod .. " + E", hl.dsp.exec_cmd(fileManager))                                                -- dots | SUPER + E   →   Open File Manager
hl.bind(mainMod .. " + space", hl.dsp.exec_cmd("zen-shell open launcher"))                                              -- dots | SUPER + Space   →   Open App Menu
hl.bind(mainMod .. " + R", hl.dsp.exec_cmd("pidof rofi && pkill rofi || rofi -show drun"))                                                       -- dots | SUPER + R   →   Open App Menu (Alt Bind)


hl.bind(mainMod .. " + SHIFT + L", hl.dsp.exec_cmd("pkill zen-shell; sleep 1; zen-shell"))                                                       -- dots | SUPER + L   →   Restart Shell / Bar



--hl.bind(mainMod .. " + V", hl.dsp.exec_cmd('bash -c ". $hypr_sources/helpers/cliphist_text"'))          -- dots | SUPER + V   →   Clipboard History (text)
--hl.bind(mainMod .. " + SHIFT + V", hl.dsp.exec_cmd('bash -c ". $hypr_sources/helpers/cliphist_image"')) -- dots | SUPER + SHIFT + V   →   Clipboard History (images/db)


--hl.bind(mainMod .. " + V", hl.dsp.exec_cmd('quickshell ipc call panel clipboard'))          -- dots | SUPER + V   →   Clipboard History (text)
--hl.bind(mainMod .. " + SHIFT + V", hl.dsp.exec_cmd('quickshell ipc call panel clipboard_images')) -- dots | SUPER + SHIFT + V   →   Clipboard History (images/db)

hl.bind(mainMod .. " + V", hl.dsp.exec_cmd("zen-shell open clipboard"))          -- dots | SUPER + V   →   Clipboard History (text)
hl.bind(mainMod .. " + SHIFT + V", hl.dsp.exec_cmd("zen-shell open clipboard_images")) -- dots | SUPER + SHIFT + V   →   Clipboard History (images)




hl.bind(mainMod .. " + SHIFT + Tab", hl.dsp.exec_cmd("win_switcher"))    -- dots | SUPER + SHIFT + Tab   →   Open Windows Switcher
hl.bind(mainMod .. " + SHIFT + H", hl.dsp.exec_cmd("shader_main reset")) -- dots | SUPER + SHIFT + H   →   Reset Shader

hl.bind(mainMod .. " +SHIFT + Return",
    hl.dsp.window.fullscreen({ -- dots | SUPER + SHIFT + Enter   →   Toggle Fullscreen
        mode = "fullscreen",
        action = "toggle",
    }))

hl.bind(mainMod .. " + SHIFT + Q", hl.dsp.exec_cmd("kill_window")) -- dots | SUPER + SHIFT + Q   →   Open Window Kill Mode


hl.bind(mainMod .. " + D", hl.dsp.exec_cmd("pidof fuzzel && pkill fuzzel || fuzzel")) -- dots | SUPER + D   →   Open App Menu (Fuzzel)


hl.bind(mainMod .. " + SHIFT + E + H", function()
    local handle = io.popen("command -v hyprshutdown >/dev/null 2>&1 && echo 'yes' || echo 'no'")  -- dots | SUPER + SHIFT + E + H   →   Shutdown / Exit Hyprland
    local result = handle:read("*a")
    handle:close()

    if result:match("yes") then
        os.execute("hyprshutdown &")
    else
        hl.dispatch(hl.dsp.exit())
    end
end, { description = "Shutdown / Exit Hyprland" })






hl.bind(mainMod .. " + C", hl.dsp.exec_cmd("zen-shell open control_center")) -- dots | SUPER + C   →   Control Center
hl.bind(mainMod .. " + grave", hl.dsp.exec_cmd("foot")) -- dots | SUPER + ~   →   Secondary Terminal


hl.bind(mainMod .. " + Tab", hl.dsp.exec_cmd("quickshell ipc call panel workspaces")) -- dots | SUPER + Tab   →   Workspace Switcher
hl.bind(mainMod .. " + A", hl.dsp.exec_cmd("zen-shell open audio")) -- dots | SUPER + A   →   Audio UI (per-app volume)







hl.bind(mainMod .. " + SHIFT + M", hl.dsp.exec_cmd("state_switcher")) -- dots | SUPER + SHIFT + M   →   Normal / Split UI State Switch
hl.bind(mainMod .. " + M", hl.dsp.exec_cmd("theme_main switch")) -- dots | SUPER + M   →   Dark / Light Mode Switch


hl.bind(mainMod .. " + W", hl.dsp.exec_cmd("zen-shell open wallpaper"))                    -- dots | SUPER + W   →   Wallpaper Picker
hl.bind(mainMod .. " + P", hl.dsp.window.pseudo())                         -- dots | SUPER + P   →   Toggle Pseudo Tiling
hl.bind(mainMod .. " + J", hl.dsp.layout("togglesplit"))                   -- dots | SUPER + J   →   Toggle Split Layout (Dwindle)


hl.bind(mainMod .. " + L", hl.dsp.exec_cmd("chrome_link_toggle"))                       -- dots | SUPER + L   →   Link/Unlink Chrome Profile

hl.bind(mainMod .. " + S", hl.dsp.workspace.toggle_special("magic"))                    -- dots | SUPER + S   →   Toggle Scratchpad (magic workspace)
hl.bind(mainMod .. " + SHIFT + S", hl.dsp.window.move({ workspace = "special:magic" })) -- dots | SUPER + SHIFT + S   →   Move window to Scratchpad

hl.bind(mainMod .. " + mouse_down", hl.dsp.focus({ workspace = "e+1" }))                -- dots | SUPER + Mouse Down   →   Next workspace
hl.bind(mainMod .. " + mouse_up", hl.dsp.focus({ workspace = "e-1" }))                  -- dots | SUPER + Mouse Up   →   Previous workspace

hl.bind(mainMod .. " + mouse:272", hl.dsp.window.drag(), { mouse = true })              -- dots | SUPER + Mouse Left   →   Drag window
hl.bind(mainMod .. " + mouse:273", hl.dsp.window.resize(), { mouse = true })            -- dots | SUPER + Mouse Right   →   Resize window
hl.bind(mainMod .. " + SHIFT + slash", hl.dsp.exec_cmd("zen-shell open keybinds")) -- dots | SUPER + SHIFT + /   →   Show Keyboard Shortcut List

hl.bind(mainMod .. " + T", hl.dsp.exec_cmd("zen-shell ipc call themes"))    -- dots | SUPER + T   →   Show Theme Cards Menu

hl.bind(mainMod .. " + ALT + S", hl.dsp.exec_cmd('sync_to_tmpfs'))





-- Laptop multimedia keys for volume and LCD brightness
hl.bind("XF86AudioRaiseVolume", hl.dsp.exec_cmd("audio_main vol up 5"), { locked = true, repeating = true })
hl.bind("XF86AudioLowerVolume", hl.dsp.exec_cmd("audio_main vol down 5"), { locked = true, repeating = true })
hl.bind("XF86AudioMute", hl.dsp.exec_cmd("audio_main vol toggle"), { locked = true, repeating = true })
hl.bind(mainMod .. " + XF86AudioRaiseVolume", hl.dsp.exec_cmd("audio_main mic up"), { locked = true, repeating = true })   -- dots | SUPER + Volume Up   →   Mic volume up
hl.bind(mainMod .. " + XF86AudioLowerVolume", hl.dsp.exec_cmd("audio_main mic down"), { locked = true, repeating = true }) -- dots | SUPER + Volume Down   →   Mic volume down
hl.bind("XF86AudioMicMute", hl.dsp.exec_cmd("audio_main mic toggle"), { locked = true, repeating = true })
hl.bind("XF86MonBrightnessUp", hl.dsp.exec_cmd("brightness_main up 5"), { locked = true, repeating = true })
hl.bind("XF86MonBrightnessDown", hl.dsp.exec_cmd("brightness_main down 5"), { locked = true, repeating = true })
hl.bind(mainMod .. " + XF86MonBrightnessUp", hl.dsp.exec_cmd("brightness_main up 1"), { locked = true, repeating = true })
hl.bind(mainMod .. " + XF86MonBrightnessDown", hl.dsp.exec_cmd("brightness_main down 1"),
    { locked = true, repeating = true })


-- Requires playerctl
hl.bind("XF86AudioNext", hl.dsp.exec_cmd("playerctl next"), { locked = true })
hl.bind("XF86AudioPause", hl.dsp.exec_cmd("playerctl play-pause"), { locked = true })
hl.bind("XF86AudioPlay", hl.dsp.exec_cmd("playerctl play-pause"), { locked = true })
hl.bind("XF86AudioPrev", hl.dsp.exec_cmd("playerctl previous"), { locked = true })

-- Switch workspaces with mainMod + [0-9]
-- Move active window to a workspace with mainMod + SHIFT + [0-9]
for i = 1, 10 do
    local key = i % 10 -- 10 maps to key 0
    hl.bind(mainMod .. " + " .. key, hl.dsp.focus({ workspace = i }))
    hl.bind(mainMod .. " + SHIFT + " .. key, hl.dsp.window.move({ workspace = i }))
    hl.bind(mainMod .. " + ALT + " .. key, hl.dsp.window.move({ workspace = i }))
end


hl.bind("caps_lock", hl.dsp.exec_cmd("caps_lock_trigger"), { locked = true })
