-- -*- mode: lua -*-
-- hl.env(name, value) ONLY sets (Hyprland 0.56 API); reading the process
-- environment is os.getenv. There is no global `env` setter.
getenv = os.getenv
local function set(k, v) hl.env(k, v) return v end

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

hl.env("rbase", "/tmp")
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

-- Locate the per-user config dir (base) that owns `sources`, mirroring .bashrc:
--   if $sources/vars is missing, take the first "$HOME"/.config/*/sources match
-- and use "$HOME/.config/<base>" as dots.
-- Prefer the value the login shell already exported (`vars` exports `base`) so
-- nothing is spawned on the normal path; otherwise replicate the .bashrc glob.
local function resolve_base()
    local d = getenv("base")
    if d and d ~= "" then return d end

    local home = getenv("HOME")
    local p = io.popen("ls -d '" .. home .. "'/.config/*/sources 2>/dev/null")
    if p then
        local first = p:read("*l")
        p:close()
        if first then return first:match("/([^/]+)/sources$") end
    end

    return "hdots"
end

local base = set("base", resolve_base())
local dots = set("dots", getenv("HOME") .. "/.config/" .. base)

-- root-daemon names are owned by dots/vars_l (a plain Lua assignment written
-- by setup/setup_vars); source it and export every var it defines so the
-- session env and the daemon agree on rdmnb/rdmnp/states.
if not pcall(dofile, getenv("HOME") .. "/.config/hdots/vars_l") then
    rdmnb  = rdmnb  or "rdmn"
    rdmnp  = rdmnp  or ("/run/" .. rdmnb)
    rstates  = rstates  or (rdmnp .. "/states")
    rstates2 = rstates2 or (rdmnp .. "/states2")
end
set("base",   base)
set("dots",   dots)
set("rdmnb",  rdmnb)
set("rdmnp",  rdmnp)
local r_daemon = set("r_daemon", rdmnp)
set("rstates",  rstates)
set("rstates2", rstates2)

local rbase = set("rbase", getenv("rbase") or "/tmp")
local rconf = set("rconf", rbase .. "/" .. getenv("USER") .. "_rconf")
local sources   = set("sources", rconf .. "/sources")
local states2   = set("states2", rconf .. "/states2")

set("default_terminal",      getenv("default_terminal") or "alacritty")
set("default_file_manager",  getenv("default_file_manager") or "thunar")
set("default_launcher",      getenv("default_launcher") or "rofi")
local auto_launch = set("auto_launch", getenv("auto_launch") or "1")

set("rcache",              rconf .. "/rcache")
set("states",              rconf .. "/states")
set("rconfig",             rconf .. "/rconfig")
set("templates",            sources .. "/templates")
set("caml",                states2 .. "/caml")
set("shaders",              getenv("HOME") .. "/.config/hypr/shaders")
set("XDG_SESSION_DESKTOP",   "Hyprland")

set("PATH", table.concat({
    sources .. "/bin",
    sources .. "/pbin",
    getenv("PATH") or "/usr/bin:/bin:/usr/local/bin",
}, ":"))