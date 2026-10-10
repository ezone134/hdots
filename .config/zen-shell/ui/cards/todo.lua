-- todo.lua — the To-Do card's entire UI layer (LuaJIT host, QUICKSHELL_MODEL
-- §Lua). The engine (Rust) is backend + renderer; this module paints the card
-- once per frame through `ctx.ui.*`, the exact same Cmd components the Rust
-- drawer emits, so geometry is byte-identical. Geometry of record is
-- draw_todo_card in src/shell/panels/cards/todo.rs
-- The parity test `the_todo_lua_module_paints_exactly_like_the_rust_drawer`
-- (src/lua/mod.rs) asserts this module produces IDENTICAL commands for every
-- state; once that holds and this module is live, the Rust drawer (and
-- todo.ron) are DELETED.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

-- glyphs (icon font code points, UTF-8 encoded as byte escapes)
local ICON_CHECK = "\239\128\140"    -- U+F00C check
local ICON_CLOSE = "\239\128\141"    -- U+F00D close
local ICON_ADD = "\239\129\167"      -- U+F067 plus
local ICON_KEYBOARD = "\239\132\156" -- U+F11C keyboard
local ICON_TODO = "\239\137\150"     -- U+F256 list-check

local RED = 0xf7768eff

-- ── utf8 helpers (LuaJIT has no utf8 stdlib) — mirror Rust chars() ─────────
local function u8len(s)
  local n = 0
  for i = 1, #s do
    local b = string.byte(s, i)
    if b < 128 or b >= 192 then n = n + 1 end
  end
  return n
end

local function is_blank(s) return string.find(s, "%S") == nil end

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h

  if not api.scene_owns_header and api.card_show_title then
    ui.title(x + s(14), y + s(9), "To-Do", fs(11.5), pal.fg)
  end

  local n = api.count
  if n > 0 then
    ui.captionr(x + w - s(14), y + s(11), api.done .. "/" .. n, fs(8.5), pal.fg3, false)
  end

  local limit = math.min(8, n)
  local start = math.min(api.scroll, math.max(0, limit - 1))
  local drawn = 0

  if n == 0 then
    ui.textc(x + w / 2, y + s(60), "No tasks yet", fs(9.5), pal.fg, false)
    ui.textc(x + w / 2, y + s(78), ICON_TODO, fs(16.0), ui.mix(pal.hover, pal.fg, 0.10), true)
  end

  if not api.scene_owns_rows then
    local bottom_line = y + h - s(40)
    for i = start, limit - 1 do
      local row_y = y + s(36) + (i - start) * s(24)
      if row_y + s(24) > y + h + 0.5 then break end
      if row_y + s(24) > bottom_line then break end
      drawn = drawn + 1
      local si = i - start
      local it = api.todos[i + 1]
      local tog_key = api.keys.toggle + si
      local del_key = api.keys.delete + si
      local row_hover = (ctx.hover == tog_key)
      local show_del = row_hover or (ctx.hover == del_key)
      if it.done then
        ui.surface(x + s(14), row_y + s(1), s(11), s(11), s(3.5), pal.acc_tint)
        ui.captionr(x + s(26.5), row_y + s(2.5), ICON_CHECK, fs(8.0), pal.acc, true)
      else
        ui.outline(
          x + s(14), row_y + s(1), s(11), s(11), s(3.5), s(1.2),
          row_hover and pal.acc or ui.mix(pal.hover, pal.fg, 0.25)
        )
      end
      ui.text(x + s(32), row_y + s(1), it.text, fs(9.5), pal.fg, false)
      if show_del then
        local dhov = (ctx.hover == del_key)
        ui.text(x + w - s(20), row_y + s(1), ICON_CLOSE, fs(9.0), dhov and RED or pal.fg, true)
      end
      ui.hit(x + s(12), row_y, w - s(24), s(24), tog_key)
      if show_del then
        ui.hit(x + w - s(24), row_y - s(4), s(22), s(22), del_key)
      end
    end
  end

  local rem = n - (start + drawn)
  if rem > 0 then
    local fy_ = y + s(36) + drawn * s(24) + s(2)
    ui.textc(x + w / 2, fy_, "+" .. rem .. " more", fs(8.0), pal.fg, false)
  end

  if not api.scene_owns_composer then
    local fy = y + h - s(38)
    local focused = (api.input ~= nil)
    local buf = api.input or ""
    ui.surface(
      x + s(12), fy, w - s(24), s(26), s(13),
      ui.mix(pal.hover, pal.fg, focused and 0.09 or 0.04)
    )
    if focused then
      ui.outline(x + s(12), fy, w - s(24), s(26), s(13), s(1.4), pal.acc)
    end
    if #buf == 0 and not focused then
      ui.text(x + s(24), fy + s(7), "Add a task…", fs(9.5), pal.fg, false)
    else
      if #buf > 0 then
        ui.text(x + s(24), fy + s(7), buf, fs(9.5), pal.fg, false)
      end
      if focused then
        local cwx = u8len(buf) * fs(9.5) * 0.62
        ui.surface(x + s(23) + cwx, fy + s(6), s(1.4), s(14), s(0.7), pal.acc)
        if not is_blank(buf) then
          ui.textr(x + w - s(22), fy + s(8), ICON_KEYBOARD, fs(8.5), pal.fg, true)
        end
      end
    end
    ui.hit(x + s(10), fy - s(4), w - s(20), s(34), api.keys.input)
    if not focused then
      local pbx = x + w - s(20)
      local pby = y + h - s(18)
      local pb_hov = (ctx.hover == api.keys.add)
      ui.surface(
        pbx - s(10), pby - s(10), s(20), s(20), s(10),
        pb_hov and pal.hover_hl or pal.hover
      )
      ui.textc(pbx, pby - s(4.5), ICON_ADD, fs(9.5), pb_hov and pal.acc or pal.fg, true)
      ui.hit(pbx - s(11), pby - s(11), s(22), s(22), api.keys.add)
    end
  end
end
