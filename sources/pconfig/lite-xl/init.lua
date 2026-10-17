-- put user settings here
-- this module will be loaded after everything else when the application starts
-- it will be automatically reloaded when saved

local core = require "core"
local keymap = require "core.keymap"
local config = require "core.config"
local style = require "core.style"
local Doc = require "core.doc"
local syntax = require "core.syntax"

--------------------------------------------------------------------------------
-- Robust Binary Resolver & Shebang / Modeline Parser (Extensionless Support)
--------------------------------------------------------------------------------

-- 1. Robust binary auto-finder for Bash, Zsh, Lua, LuaJIT, and Rust toolchains
local function resolve_binaries()
    local targets = {
        bash   = {"bash", "/bin/bash", "/usr/bin/bash"},
        zsh    = {"zsh", "/bin/zsh", "/usr/bin/zsh"},
        lua    = {"lua", "/usr/bin/lua", "/usr/local/bin/lua"},
        luajit = {"luajit", "/usr/bin/luajit", "/usr/local/bin/luajit"},
        rustc  = {"rustc", "/usr/bin/rustc", "/usr/local/bin/rustc", (os.getenv("HOME") or "") .. "/.cargo/bin/rustc"},
        cargo  = {"cargo", "/usr/bin/cargo", "/usr/local/bin/cargo", (os.getenv("HOME") or "") .. "/.cargo/bin/cargo"}
    }

    local resolved = {}
    for name, candidates in pairs(targets) do
        local found = nil
        for _, cand in ipairs(candidates) do
            if cand:find("/") then
                local f = io.open(cand, "r")
                if f then f:close(); found = cand; break end
            else
                local handle = io.popen("command -v " .. cand .. " 2>/dev/null")
                if handle then
                    local res = handle:read("*a"):gsub("%s+", "")
                    handle:close()
                    if res ~= "" then found = res; break end
                end
            end
        end
        resolved[name] = found
    end
    return resolved
end

BIN = resolve_binaries()

-- 2. Shebang, Emacs (-*- mode: ... -*-), and Vim (vim: ft=...) header parser
local function detect_syntax_from_header(doc)
    local header = ""
    for i = 1, math.min(3, doc:get_line_count()) do
        header = header .. (doc:get_line(i) or "") .. "\n"
    end

    local lang = nil

    -- Check Emacs modeline
    local mode = header:match("%-%*%-.-mode:%s*([%w_%-]+).-%-%*%-")
    if mode then
        mode = mode:lower()
        if mode == "sh" or mode == "bash" or mode == "zsh" then lang = "Shell"
        elseif mode == "lua" then lang = "Lua"
        elseif mode == "rust" then lang = "Rust" end
    end

    -- Check Vim modeline
    if not lang then
        local vim_ft = header:match("vim:%s*ft=([%w_%-]+)") or header:match("vim:%s*syntax=([%w_%-]+)")
        if vim_ft then
            vim_ft = vim_ft:lower()
            if vim_ft == "sh" or vim_ft == "bash" or vim_ft == "zsh" then lang = "Shell"
            elseif vim_ft == "lua" then lang = "Lua"
            elseif vim_ft == "rust" then lang = "Rust" end
        end
    end

    -- Check Standard Shebang
    if not lang then
        local shebang = header:match("^#![^\r\n]+")
        if shebang then
            shebang = shebang:lower()
            if shebang:find("lua") or shebang:find("luajit") then lang = "Lua"
            elseif shebang:find("bash") or shebang:find("zsh") or shebang:find("sh") then lang = "Shell"
            elseif shebang:find("rust") then lang = "Rust" end
        end
    end

    if lang then
        local syn = syntax.get(lang)
        if syn then doc:set_syntax(syn) end
    end
end

-- Hook into file loading to automatically evaluate headers for extensionless files
local orig_load = Doc.load
function Doc:load(filename)
    orig_load(self, filename)
    if self.syntax and self.syntax.name == "Plain Text" then
        detect_syntax_from_header(self)
    end
end


------------------------------ Themes ----------------------------------------

-- light theme:
-- core.reload_module("colors.summer")

--------------------------- Key bindings -------------------------------------

-- key binding:
-- keymap.add { ["ctrl+escape"] = "core:quit" }

-- pass 'true' for second parameter to overwrite an existing binding
-- keymap.add({ ["ctrl+pageup"] = "root:switch-to-previous-tab" }, true)
-- keymap.add({ ["ctrl+pagedown"] = "root:switch-to-next-tab" }, true)

------------------------------- Fonts ----------------------------------------

-- customize fonts:
-- style.font = renderer.font.load(DATADIR .. "/fonts/FiraSans-Regular.ttf", 14 * SCALE)
-- style.code_font = renderer.font.load(DATADIR .. "/fonts/JetBrainsMono-Regular.ttf", 14 * SCALE)
--
-- DATADIR is the location of the installed Lite XL Lua code, default color
-- schemes and fonts.
-- USERDIR is the location of the Lite XL configuration directory.
--
-- font names used by lite:
-- style.font        : user interface
-- style.big_font    : big text in welcome screen
-- style.icon_font   : icons
-- style.icon_big_font : toolbar icons
-- style.code_font   : code
--
-- the function to load the font accept a 3rd optional argument like:
--
-- {antialiasing="grayscale", hinting="full", bold=true, italic=true, underline=true, smoothing=true, strikethrough=true}
--
-- possible values are:
-- antialiasing: grayscale, subpixel
-- hinting: none, slight, full
-- bold: true, false
-- italic: true, false
-- underline: true, false
-- smoothing: true, false
-- strikethrough: true, false

------------------------------ Plugins ----------------------------------------

-- disable plugin loading setting config entries:

-- disable plugin detectindent, otherwise it is enabled by default:
-- config.plugins.detectindent = false

---------------------------- Miscellaneous -------------------------------------

-- modify list of files to ignore when indexing the project:
-- config.ignore_files = {
--   -- folders
--   "^%.svn/",        "^%.git/",   "^%.hg/",        "^CVS/", "^%.Trash/", "^%.Trash%-.*/",
--   "^node_modules/", "^%.cache/", "^__pycache__/",
--   -- files
--   "%.pyc$",         "%.pyo$",    "%.exe$",        "%.dll$",   "%.obj$", "%.o$",
--   "%.a$",           "%.lib$",    "%.so$",         "%.dylib$", "%.ncb$", "%.sdf$",
--   "%.suo$",         "%.pdb$",    "%.idb$",        "%.class$", "%.psd$", "%.db$",
--   "^desktop%.ini$", "^%.DS_Store$", "^%.directory$",
-- }