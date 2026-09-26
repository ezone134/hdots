local env, getenv = hl.env, os.getenv
local function set(k, v) env(k, v) return v end

local override_path = getenv("HOME") .. "/env_override"

-- Check if override file exists; create template if missing, dofile if it exists
local f = io.open(override_path, "r")
if not f then
    f = io.open(override_path, "w")
    if f then
        f:write([[
-- Do not edit env.lua, edit this instead
-- Lua environment overrides
-- Uncomment and edit the values below to override them

-- hl.env("rbase", "/tmp")
-- hl.env("default_terminal", "alacritty")
-- hl.env("default_file_manager", "thunar")
-- hl.env("default_launcher", "rofi")
-- hl.env("auto_launch", "1")
]])
        f:close()
    end
else
    f:close()
    pcall(dofile, override_path)
end

-- Path definitions
local rbase = set("rbase", getenv("rbase") or "/tmp")
local rconf = set("rconf", rbase .. "/" .. getenv("USER") .. "_rconf")
local sources   = set("sources", rconf .. "/sources")
local states2   = set("states2", rconf .. "/states2")
local dots      = set("dots", getenv("HOME") .. "/.config/hdots")

set("default_terminal",      getenv("default_terminal") or "alacritty")
set("default_file_manager",  getenv("default_file_manager") or "thunar")
set("default_launcher",      getenv("default_launcher") or "rofi")
local auto_launch = set("auto_launch", getenv("auto_launch") or "1")

set("r_daemon",           rbase .. "/r_daemon")
set("rcache",            rconf .. "/rcache")
set("states",                rconf .. "/states")
set("rconfig",              rconf .. "/rconfig")
set("templates",             sources .. "/templates")
set("custom_acc_master_list",states2 .. "/custom_acc_master_list")
set("shaders",               getenv("HOME") .. "/.config/hypr/shaders")
set("XDG_SESSION_DESKTOP",   "Hyprland")

set("PATH", table.concat({
    sources .. "/bin",
    sources .. "/bin2",
    getenv("PATH") or "/usr/bin:/bin:/usr/local/bin",
}, ":"))
