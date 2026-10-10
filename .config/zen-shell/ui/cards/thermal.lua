-- thermal.lua — the Thermal zones card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). Geometry of record is thermal.ron (the live,
-- zero-`Ink` scene):
--   Header     glyph U+F2C9 fs12 fg2 at pad, title "Thermal zones" semibold
--              11.5 at pad+17, meta "{n} zones" caption fg3 right
--   Text       "no /sys/class/thermal zones" centered (only while empty)
--   Grid       thermal_cells: 3-wide chips, gap 2, cell h 24, r 5, from
--              y+26, box h −2 above the card bottom, keys 0..n — each cell
--              the resting hover-well surface + zone caption (Ui/Regular
--              7.5, the `caption` voice) centered at cy+3 and the temp
--              (Ui/Regular 9) centered at cy+12 at its warn/crit ink
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h
  local p = s(12)

  if api.card_show_glyph then
    ui.text(x + p, y + s(8), api.glyph, fs(12), pal.fg2, true)
  end
  if api.card_show_title then
    ui.title(x + p + s(17), y + s(9), "Thermal zones", fs(11.5), pal.fg)
  end
  if api.meta ~= "" then
    ui.captionr(x + w - p, y + s(11), api.meta, fs(8.5), pal.fg3, false)
  end

  if api.thermal_empty then
    ui.textc(x + w / 2, y + h / 2, "no /sys/class/thermal zones", fs(9), pal.fg3, false)
    return
  end

  if api.thermal_live then
    local cell_w = (w - s(4)) / 3
    local cell_h = s(24)
    local gap = s(2)
    local grid_y = y + s(26)
    local i = 0
    while i < #api.cells do
      local local_i = i
      local col = local_i % 3
      local row = math.floor(local_i / 3)
      local cx = x + col * (cell_w + gap)
      local cy = grid_y + row * (cell_h + gap)
      if cy > y + h + 0.5 then break end
      local cell = api.cells[local_i + 1]
      ui.surface(cx, cy, cell_w, cell_h, s(5), pal.hover)
      ui.captionc(cx + cell_w / 2, cy + s(3), cell.label, fs(7.5), pal.fg, false)
      ui.captionc(cx + cell_w / 2, cy + s(12), cell.sub, fs(9), cell.sub_color, false)
      ui.hit(cx, cy, cell_w, cell_h, i)
      i = i + 1
    end
  end
end