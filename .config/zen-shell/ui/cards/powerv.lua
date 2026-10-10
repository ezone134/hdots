-- powerv.lua — the vertical Power card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). The engine (Rust) is backend + renderer; this module
-- paints the card once per frame through `ctx.ui.*`, the exact same Cmd
-- components the scene emitted, so geometry is byte-identical. Geometry of
-- record is ui/cards/powerv.ron (the live zero-`Ink` scene):
--   Header  glyph U+F0E7 @ (12, 8) fg2 12, title "Power" @ (12+17, 9) semibold
--           11.5 fg
--   Column  below the 30 px header, `halign: center` / `valign: middle` — five
--           32 px buttons on a 41 px pitch (9 px gaps), block 196 px tall, each
--           centered in the card width
--   Button  Stack: Surface 32×32 r10 (fill `power_bg_i`, 1 px hairline), the
--           hold-to-confirm liquid (a `Bar` x:2 y:-2 w:28 h:32 r8, vertical,
--           bottom-up, gated on `power_hold_on_i`), the glyph (14 px centered,
--           ink `power_ink_i`) and the Hit (key 12000+i)
-- The parity test `the_powerv_lua_module_paints_exactly_like_the_scene`
-- (src/lua/mod.rs) asserts this module produces IDENTICAL commands for every
-- state; once it holds and this module is live, powerv.ron is DELETED.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

local ICON_BOLT = "\239\131\167" -- U+F0E7
local KEYS_BASE = 12000
-- lock · logout · sleep · restart · shutdown
local GLYPHS = {
  "\239\128\163", -- U+F023
  "\239\130\139", -- U+F08B
  "\239\134\134", -- U+F186
  "\239\128\161", -- U+F021
  "\239\128\145", -- U+F011
}

function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h

  -- header — the bolt glyph shifts the title right by 17
  local hdr = s(30)
  local p = s(12)
  local tx = p
  if api.card_show_glyph then
    ui.text(x + p, y + s(8), ICON_BOLT, fs(12.0), pal.fg2, true)
    tx = p + s(17)
  end
  if api.card_show_title then
    ui.title(x + tx, y + s(9), "Power", fs(11.5), pal.fg)
  end

  -- one centered column: block centered in the body below the header
  local d = s(32)
  local gap = s(9)
  local block = 5 * d + 4 * gap
  local body_h = h - hdr
  local bx = x + (w - d) / 2
  local by0 = y + hdr + math.max(body_h - block, 0) / 2
  for i = 1, 5 do
    local b = api.buttons[i]
    local by = by0 + (i - 1) * (d + gap)
    local cx = bx + d / 2
    local cy = by + d / 2
    ui.surface(bx, by, d, d, s(10), b.bg)
    ui.outline(bx, by, d, d, s(10), s(1.0), pal.hairline)
    if b.on then
      ui.bar(bx + s(2), by - s(2), s(28), s(32), s(8), tonumber(b.hold), 1.0,
        api.hold_fill, pal.clear, true)
    end
    ui.textc(cx, cy, GLYPHS[i], fs(14.0), b.ink, true)
    ui.hit(bx, by, d, d, KEYS_BASE + (i - 1))
  end
end
