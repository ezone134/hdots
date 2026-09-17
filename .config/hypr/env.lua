--- Mimicking your shell logic ---
local home = os.getenv("HOME")
local user = os.getenv("USER")

-- Basic Defaults
local hypr_base = os.getenv("hypr_base") or "/tmp"
local default_terminal = os.getenv("default_terminal") or "alacritty"
local default_file_manager = os.getenv("default_file_manager") or "thunar"
local default_launcher = os.getenv("default_launcher") or "rofi"
local hypr_auto_launch = os.getenv("hypr_auto_launch") or "1"

-- Exported Variables
local hypr_conf = hypr_base .. "/" .. user .. "_hypr_conf"
-- Define hypr_cache variable explicitly to avoid nil errors
local hypr_cache = hypr_conf .. "/hypr_cache"
hl.env("hypr_base", hypr_base)
hl.env("default_terminal", default_terminal)
hl.env("default_file_manager", default_file_manager)
hl.env("default_launcher", default_launcher)
hl.env("hypr_auto_launch", hypr_auto_launch)

hl.env("rnd", "$SRANDOM")
hl.env("hdots", home .. "/.config/hdots")
hl.env("hdots_sources", home .. "/.config/hdots/sources")
hl.env("hypr_conf", hypr_conf)
hl.env("hypr_log", hypr_conf .. "/log")
hl.env("lock_files", hypr_conf .. "/lock_hold_files")
hl.env("hypr_scripts", hypr_conf .. "/scripts")
hl.env("hypr_global", hypr_base .. "/hypr_global")
hl.env("hypr_cache", hypr_cache) -- Safely enabled now
hl.env("states", hypr_conf .. "/states")
local states2 = hypr_conf .. "/states2"
hl.env("states2", states2)
hl.env("hypr_temp", hypr_conf .. "/hypr_temp")
hl.env("hypr_sources", hypr_conf .. "/sources")
hl.env("hypr_bin", hypr_conf .. "/hypr_bin")

-- QML import path so quickshell can load compiled (.so) modules from its own
-- config tree. Resolves through the ~/.config/quickshell symlink, so it tracks
-- whichever copy is live (hdots now, tmpfs during a session).
-- modules/ = QML panels; backends/ = compiled .so modules (QsIo, QsHypr).
hl.env("QML_IMPORT_PATH", home .. "/.config/quickshell/modules:" .. home .. "/.config/quickshell/backends")

-- Paths using the defined hypr_cache variable
hl.env("custom_acc_master_list", states2 .. "/custom_acc_master_list")

hl.env("hex_states", hypr_conf .. "/theme_hex_states")
hl.env("theme_elements", hypr_conf .. "/theme_elements_states")
hl.env("shaders", home .. "/.config/hypr/shaders")

-- Paths using the defined hypr_cache variable
hl.env("rofi_pid", states2 .. "/rofi_pop_pid")
hl.env("rofi_id", states2 .. "/rofi_id")
hl.env("XDG_SESSION_DESKTOP", "Hyprland")



-- Path variables
local current_path = os.getenv("PATH") or "/usr/bin:/bin:/usr/local/bin"
local custom_path = hypr_conf .. "/hypr_bin"

local new_path = table.concat({
    "/tmp/boss-bins",
    custom_path,
    home .. "/bin",
    home .. "/.local/bin",
    home .. "/.pixi/bin",
    current_path
}, ":")

hl.env("PATH", new_path)
