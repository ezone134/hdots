-- water.lua — the Water card's entire UI layer (LuaJIT host, QUICKSHELL_MODEL
-- §Lua). The engine (Rust) is backend + renderer; this module paints the card
-- once per frame through `ctx.ui.*`, the exact same Cmd components the scene
-- emits, so geometry is byte-identical. Geometry of record is ui/cards/water.ron
-- (the live, zero-`Ink` scene):
--   Header  glyph (12, 8) fg2, title "Water" @ (12+17, 9) 11.5, meta fg3 right
--   Ring    water_frac: card-centred, cy = h/2 − 7, r (h/2…) fit, dot 4/3.5
--   Text    `{water_hero}` display/medium centred (x + 60), cy = h/2 − 13
--   Text    `{water_goal_txt}` fg3 centred, cy = h/2 + 1
--   Hits    − (31 301) · + glass (31 300), 46×22 accent/hover chips, 6.25 down
-- The parity test `the_water_lua_module_paints_exactly_like_the_scene`
-- (src/lua/mod.rs) asserts this module produces IDENTICAL commands for every
-- state; once it holds and this module is live, water.ron is DELETED.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

local ICON_WATER = "\238\144\176" -- U+E430 brightness_high (the drawer's mark)
local MINUS = "\226\136\146" -- U+2212

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h

  -- header — glyph shifts the title right by 17
  local p = s(12)
  local tx = p
  if api.card_show_glyph then
    ui.text(x + p, y + s(8), ICON_WATER, fs(12.0), pal.fg2, true)
    tx = p + s(17)
  end
  if api.card_show_title then
    ui.title(x + tx, y + s(9), "Water", fs(11.5), pal.fg)
  end
  if api.water_meta ~= "" then
    ui.captionr(x + w - p, y + s(11), api.water_meta, fs(8.5), pal.fg3, false)
  end

  -- bead-ring gauge — card-centred, sized by the `Half` fit (offset 62)
  local base_r = (h / ctx.grid - 62.0) / 2.0
  if base_r < 20.0 then base_r = 20.0 elseif base_r > 34.0 then base_r = 34.0 end
  local base_d = (base_r >= 26.0) and 4.0 or 3.5
  ui.ring(x + w / 2, y + h / 2 - s(7), s(base_r), s(base_d), 36, api.water_lit, api.water_ring_ink, api.water_dim)

  -- hero readout + goal caption — both centered on the 120px box from x
  local box_cx = x + s(60)
  local hsz = (h >= s(114)) and 14 or 12
  ui.heroc(box_cx, y + h / 2 - s(13), api.water_hero, fs(hsz), api.water_hero_ink, false)
  ui.captionc(box_cx, y + h / 2 + s(1), api.water_goal_txt, fs(7.5), pal.fg3, false)

  -- − / + glass buttons, centered on the card, 10px above the bottom
  local bw, bh, sp = s(46), s(22), s(12)
  local block = 2 * bw + sp
  local b0 = x + math.max(w - block, 0) / 2
  local by = (y + h - s(32)) + s(6.25)
  ui.surface(b0, by, bw, bh, s(6), api.water_dec_bg)
  ui.textc(b0 + bw / 2, by, MINUS, fs(9.5), pal.fg, false)
  ui.hit(b0, by, bw, bh, 31301)
  local b1 = b0 + bw + sp
  ui.surface(b1, by, bw, bh, s(6), api.water_inc_bg)
  ui.textc(b1 + bw / 2, by, "+ glass", fs(9.5), pal.acc, false)
  ui.hit(b1, by, bw, bh, 31300)
end
