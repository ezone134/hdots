-- systemdunits.lua — the Services card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). The engine (Rust) is backend + renderer; this module
-- paints the card once per frame through `ctx.ui.*` — the exact same Cmd
-- components the scene emits, so geometry is byte-identical. Geometry of record
-- was ui/cards/systemdunits.ron (the live, zero-`Ink` scene):
--   Header  title "Services" semibold 11.5, glyph `{sus_icon}` (check / shield)
--           fg2, meta `{sus_meta}` ("{n} failed") caption fg3 right at pad 12
--   Rows    sus_rows: newest-first raised chips from y+28, 22.5 px stride (r 5)
--           — danger name from 6 in, truncated 20 px short of the card's right
--           edge, right ✕ glyph (danger, icon) 14 in from the card edge
--   Text    "all services up" caption fg3 centered (only while empty)
--   Hit     bottom-right refresh well (key 31990), raised → raised_hl, fg3 → acc
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h
  local p = s(12)

  local tx = p
  if api.card_show_glyph then
    ui.text(x + p, y + s(8), api.sus_icon, fs(12), pal.fg2, true)
    tx = p + s(17)
  end
  if api.card_show_title then
    ui.title(x + tx, y + s(9), "Services", fs(11.5), pal.fg)
  end
  if api.sus_meta ~= "" then
    ui.captionr(x + w - p, y + s(11), api.sus_meta, fs(8.5), pal.fg3, false)
  end

  if not api.sus_live then
    ui.textc(x + w / 2, y + h / 2, "all services up", fs(9), pal.fg3, false)
  else
    local row_h = s(19)
    local pitch = s(19)
    local row_w = w - p * 2
    local avail = w - p * 2 - s(20)
    local gp = 0.62 * fs(8.5)
    if gp < 1 then gp = 1 end
    local i = 0
    while i < #api.rows do
      local ry = y + s(28) + i * pitch
      if ry + row_h > y + h - s(28) + 0.5 then break end
      local row = api.rows[i + 1]
      -- raised chip well behind every row
      ui.surface(x + p, ry, row_w, row_h, s(5), pal.raised)
      local t = row.unit
      local n = math.floor(avail / gp)
      if n < 8 then n = 8 end
      if ui.len(t) > n then t = ui.sub(t, n) end
      ui.caption(x + p + s(6), ry, t, fs(8.5), row.color, false)
      ui.captionr(x + w - s(14), ry, row.icon, fs(9), row.color, true)
      i = i + 1
    end
  end

  local key = 31990
  local hov = (ctx.hover == key)
  local bx = x + w - s(45)
  local by = y + s(29.5)
  local bw = s(30)
  local bh = s(20)
  ui.surface(bx, by, bw, bh, s(6), hov and pal.raised_hl or pal.raised)
  ui.textc(bx + bw / 2, by, "\239\128\161", fs(9), hov and pal.acc or pal.fg3, true)
  ui.hit(bx, by, bw, bh, key)
end
