-- batteryv.lua — the vertical Battery card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). The engine (Rust) is backend + renderer; this
-- module paints the card once per frame through `ctx.ui.*`, the exact same Cmd
-- components the scene emits, so geometry is byte-identical. Geometry of
-- record is ui/cards/batteryv.ron (the live, zero-`Ink` scene):
--   Header    glyph @ (12, 8) fg2, title @ (29, 9) semibold 11.5
--   Battery   card-fit upright gauge, percent line anchored y−24 bottom
--   Rows      four metric rows (x 30, stride 18, bottom 26)
--   Hit       power-save pill (12, h−24, w−50, 22, r 11, key 32130)
--   Dots      one per pane, only while the cursor rests on the card
-- The parity test `the_batteryv_lua_module_paints_exactly_like_the_scene`
-- (src/lua/mod.rs) asserts this module produces IDENTICAL commands for every
-- state; once it holds and this module is live, batteryv.ron is DELETED.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

-- glyph (icon font code point, UTF-8 byte escape) — the header mark
local ICON_BATTERY = "\239\137\128" -- U+F240 battery-full

local function clamp(v, lo, hi)
  if v < lo then return lo end
  if v > hi then return hi end
  return v
end

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h

  -- header (always) — glyph shifts the title right by 17
  local p = s(12)
  local tx = p
  if api.card_show_glyph then
    ui.text(x + p, y + s(8), ICON_BATTERY, fs(12.0), pal.fg2, true)
    tx = p + s(17)
  end
  if api.card_show_title then
    ui.title(x + tx, y + s(9), "Battery", fs(11.5), pal.fg)
  end

  local pane0 = api.present and api.pane == 0
  local pane1 = api.present and api.pane == 1

  -- ── pane 0: upright gauge + percent ──
  if pane0 then
    local aw = clamp(w * 0.30, 16.0, 70.0)
    local ah = math.max(h - s(44), aw * 1.6)
    local iw = math.max(aw * 2.0 / 3.0, 12.0)
    local cx = x + w / 2
    local cy = y + s(38) + (ah - iw * 1.7) / 2
    ui.battery(cx, cy, iw, iw * 1.7, true, api.pct, pal.fg2, pal.hover, api.ink)
    ui.textc(x + w / 2, y + h - s(24), api.pct_txt, fs(11.0), api.ink, false)
  end

  -- ── pane 1: metric rows + power-save pill + its label ──
  if pane1 then
    local bottom_line = y + h - s(26)
    local ry = y + s(30)
    for i = 1, 4 do
      if ry + s(18) > y + h + 0.5 then break end
      if ry + s(18) > bottom_line then break end
      local row = api.rows[i]
      ui.caption(x + s(12), ry, row.label, fs(9.0), pal.fg3, false)
      ui.caption(x + w - s(38), ry, row.value, fs(9.5), pal.fg, false)
      ry = ry + s(18)
    end
    ui.surface(x + s(12), y + h - s(24), w - s(50), s(22), s(11), api.psave_bg)
    ui.hit(x + s(12), y + h - s(24), w - s(50), s(22), 32130)
    ui.text(x + s(36), y + h - s(17.5), api.psave_lbl, fs(9.0), api.psave_ink, false)
  end

  -- ── the no-battery state (the drawer's early return) ──
  if not api.present then
    ui.textc(x + w / 2, y + h / 2 - s(6), "no battery", fs(9.0), pal.fg3, false)
  end

  -- ── pane dots (only while the cursor rests on the card) ──
  if api.hover_dots then
    local step = s(7)
    local dot = step * 0.66
    local total = 2 * step
    local dx = x + w / 2 - total / 2
    local dyy = y + h - s(3.5) - dot
    for i = 0, 1 do
      local c = (i == api.pane) and pal.acc or pal.hover
      ui.surface(dx + i * step, dyy, dot, dot, dot / 2, c)
    end
  end
end
