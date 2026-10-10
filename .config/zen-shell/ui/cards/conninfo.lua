-- conninfo.lua — the Network card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). The engine (Rust) is backend + renderer; this module
-- paints the card once per frame through `ctx.ui.*` — the exact same Cmd
-- components the scene emits, so geometry is byte-identical. Geometry of record
-- was ui/cards/conninfo.ron (the live, zero-`Ink` scene):
--   Header  title "Network" semibold 11.5, glyph U+E250 fg2, meta
--           `{conn_meta}` ("{n} addrs") caption fg3 right at pad 12
--   Rows    conn_rows_v: iface/gateway/dns rows from y+30, 18 px stride — label
--           caption at the pad (fg, gateway/dns dimmed fg3), mono value right at
--           14 from the card edge
--   Text    "no network interfaces" caption fg3 centered (only while empty)
--   Hit     bottom-right refresh well (key 31960), raised → raised_hl, fg3 → acc
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h
  local p = s(12)

  -- header — glyph shifts the title right by 17, meta drops when empty
  local tx = p
  if api.card_show_glyph then
    ui.text(x + p, y + s(8), "\238\137\144", fs(12), pal.fg2, true)
    tx = p + s(17)
  end
  if api.card_show_title then
    ui.title(x + tx, y + s(9), "Network", fs(11.5), pal.fg)
  end
  if api.conn_meta ~= "" then
    ui.captionr(x + w - p, y + s(11), api.conn_meta, fs(8.5), pal.fg3, false)
  end

  if not api.conn_live then
    ui.textc(x + w / 2, y + h / 2, "no network interfaces", fs(9), pal.fg3, false)
  else
    local row_h = s(18)
    local pitch = s(18)
    local i = 0
    while i < #api.rows do
      local ry = y + s(30) + i * pitch
      if ry + row_h > y + h - s(32) + 0.5 then break end
      local row = api.rows[i + 1]
      ui.caption(x + p, ry, row.label, fs(8.5), row.lcolor, false)
      ui.monor(x + w - s(14), ry, row.value, fs(8.5), row.vcolor)
      i = i + 1
    end
  end

  -- refresh well — right edge 45 in, top 34.5 down (region_dy 2.5), 30×22
  local key = 31960
  local hov = (ctx.hover == key)
  local bx = x + w - s(45)
  local by = y + s(34.5)
  local bw = s(30)
  local bh = s(22)
  ui.surface(bx, by, bw, bh, s(6), hov and pal.raised_hl or pal.raised)
  ui.textc(bx + bw / 2, by, "\239\128\161", fs(9), hov and pal.acc or pal.fg3, true)
  ui.hit(bx, by, bw, bh, key)
end
