-- eyerest.lua — the Eye rest card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). The engine (Rust) is backend + renderer; this
-- module paints the card once per frame through `ctx.ui.*`, the exact same Cmd
-- components the scene emits, so geometry is byte-identical. Geometry of
-- record is ui/cards/eyerest.ron (the live, zero-`Ink` scene):
--   Header  glyph (12, 8) fg2, title "Eye rest" @ (12+17, 9) 11.5, meta fg3 right
--   Ring    er_frac: cx (12+28), cy (24+26), r 23, dot 3, 36 beads
--   Text    `{er_hero}` display/medium @ (12+28+23+12, 43) 21px
--   Text    `{er_state}` fg3 @ (75, 60) 8.5 · hint "20 min focus · 20 s rest" @ (12, 79) 8
--   Hits    Start/… (31 900) · Reset (31 901), 52×22 chips, 12 in, 10 above bottom
-- The parity test `the_eyerest_lua_module_paints_exactly_like_the_scene`
-- (src/lua/mod.rs) asserts this module produces IDENTICAL commands for every
-- state; once it holds and this module is live, eyerest.ron is DELETED.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

local ICON_TIMER = "\239\149\179" -- U+F573 timer (the drawer's mark)
local DOT = "\194\183" -- U+00B7 (the hint's mid-dot)

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h

  -- header — glyph shifts the title right by 17
  local p = s(12)
  local tx = p
  if api.card_show_glyph then
    ui.text(x + p, y + s(8), ICON_TIMER, fs(12.0), pal.fg2, true)
    tx = p + s(17)
  end
  if api.card_show_title then
    ui.title(x + tx, y + s(9), "Eye rest", fs(11.5), pal.fg)
  end
  if api.er_meta ~= "" then
    ui.captionr(x + w - p, y + s(11), api.er_meta, fs(8.5), pal.fg3, false)
  end

  -- bead-ring gauge — fixed 40/50 anchor, r 23, dot 3
  ui.ring(x + s(40), y + s(50), s(23), s(3), 36, api.er_lit, api.er_ring_ink, pal.fg3)

  -- hero mm:ss readout + phase word — both left-aligned on the 75px rail
  ui.hero(x + s(75), y + s(43), api.er_hero, fs(21.0), api.er_hero_ink)
  ui.caption(x + s(75), y + s(60), api.er_state, fs(8.5), pal.fg3, false)
  -- hint chip under the ring
  ui.caption(x + s(12), y + s(79), "20 min focus " .. DOT .. " 20 s rest", fs(8.0), pal.fg3, false)

  -- toggle (accent) + reset chips: 52×22, 12 in from the edges, 10 above bottom
  local by = y + h - s(32)
  ui.surface(x + p, by, s(52), s(22), s(6), api.er_tog_bg)
  ui.textc(x + p + s(52) / 2, by, api.er_tog_lbl, fs(9.5), pal.acc, false)
  ui.hit(x + p, by, s(52), s(22), 31900)
  local rx = x + w - p - s(52)
  ui.surface(rx, by, s(52), s(22), s(6), api.er_rst_bg)
  ui.textc(rx + s(52) / 2, by, "Reset", fs(9.5), pal.fg, false)
  ui.hit(rx, by, s(52), s(22), 31901)
end