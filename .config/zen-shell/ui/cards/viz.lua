-- viz.lua — the Visualizer card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). The engine (Rust) is backend + renderer; this module
-- paints the card once per frame through `ctx.ui.*`, the exact same Cmd
-- components the scene emitted, so geometry is byte-identical. The geometry
-- this module reproduces was ui/cards/viz.ron (the live zero-`Ink` scene),
-- itself the retired `draw_viz_card` body:
--   Header   glyph U+F001 fs12 acc at pad 12 (Light), title "Visualizer"
--            semibold fs12 at pad+18, meta none
--   Hit      the style cycle: a 16×20 well 4 in from the right edge / 4 down,
--            glyph U+F009 fs12 (fg, acc on hover), key 33000
--   Spectrum one bound series over the three styles, kind-gated on
--            `viz_style`; pitch is `plot_w / n` (NOT a line trace's
--            `w / (n-1)`), bars stand on the plot's bottom edge, every 4th
--            takes the flat accent and the rest mix toward fg, and sub-0.5 px
--            bars are culled. The plot box is 16 in from each side, 42 down,
--            12 up from the card's bottom (the drawer's own `.max(20)` /
--            `.max(12)` floors).
--   Caption  the "no audio" line (fg3 fs8, centered 24 up from the bottom),
--            only on a card tall enough to carry it (h ≥ s(90)).
-- The parity test `the_viz_lua_module_paints_exactly_like_the_scene`
-- (src/lua/mod.rs) asserts this module produces IDENTICAL commands for every
-- state; once it holds and this module is live, viz.ron is DELETED.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

local ICON_NOTE = "\239\128\129" -- U+F001
local ICON_ROWS = "\239\128\137" -- U+F009

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h

  -- header — the note glyph shifts the title right to pad+18
  if api.card_show_glyph then
    ui.text(x + s(12), y + s(8), ICON_NOTE, fs(12), pal.acc, true)
  end
  if api.card_show_title then
    local tx = s(12)
    if api.card_show_glyph then tx = s(30) end
    ui.title(x + tx, y + s(9), "Visualizer", fs(12), pal.fg)
  end

  -- the style cycle (top-right, icon only — no pill bg)
  local hov = ctx.hover == api.key
  ui.text(x + w - s(18), y + s(9), ICON_ROWS, fs(12), hov and pal.acc or pal.fg, true)
  ui.hit(x + w - s(20), y + s(4), s(16), s(20), api.key)

  -- the plot box: the drawer's own arithmetic and floors
  local area_x = x + s(16)
  local area_w = w - s(32)
  if area_w < 20 then area_w = 20 end
  local area_top = y + s(42)
  local area_bot = y + h - s(12)
  local area_h = area_bot - area_top
  if area_h < 12 then area_h = 12 end

  -- one Spectrum per style; only the active one paints. The series length IS
  -- the bar count, so an empty capture paints nothing.
  local data = api.series
  if api.style == 1 then
    -- wave: a line through the step midpoints, flat accent
    ui.spectrum(area_x, area_top, area_w, area_h, data, "wave",
      0.5, 0.0, s(0), s(0), s(2), s(1), pal.acc, 0, 0.0)
  elseif api.style == 2 then
    -- blocks: bars inset 1 from both step edges, 1 px cap
    ui.spectrum(area_x, area_top, area_w, area_h, data, "block",
      0.5, 0.0, s(1), s(1), s(2), s(1), pal.acc, 4, 0.4)
  else
    -- rounded bars (default): half-step wide, fully-round cap
    ui.spectrum(area_x, area_top, area_w, area_h, data, "rounded",
      0.5, 0.5, s(0), s(2), s(2), s(1), pal.acc, 4, 0.35)
  end

  -- the silent caption only earns its place on a tall-enough card
  if api.silent and h >= s(90) then
    ui.textc(x + w / 2, y + h - s(24), "no audio", fs(8), pal.fg3, false)
  end
end
