-- procmon.lua — the Processes card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). Geometry of record was ui/cards/procmon.ron (the
-- live, zero-`Ink` scene):
--   Header  title "Processes" semibold 11.5, mono meta "CPU {n}%" fg3 right
--           at pad 12
--   Rows    top_procs_rows from y+32 on a 22 px pitch, pad 14: fg label at 9 px
--           truncated 64 px · right-anchored 80 px meter bar (4 px tall, top 14,
--           r 2) whose fraction is ticks / top ticks, the fill acc over a hover
--           track.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px, `ctx.fs(n)` scales a
-- base font size (rounded, matching the engine fs()).

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h
  local p = s(12)

  if api.card_show_title then
    ui.title(x + p, y + s(9), "Processes", fs(11.5), pal.fg)
  end
  if api.meta ~= "" then
    ui.monor(x + w - p, y + s(11), api.meta, fs(8.5), pal.fg3)
  end

  local row_h = s(22)
  local pitch = s(22)
  local pmin = s(14)
  local avail = w - pmin * 2 - s(64)
  local gp = 0.62 * fs(9.0)
  if gp < 1 then gp = 1 end
  local bw = s(80)
  local bl = x + w - s(14) - bw
  local i = 0
  while i < #api.rows do
    local ry = y + s(32) + i * pitch
    if ry + row_h <= y + h + 0.5 then
      local row = api.rows[i + 1]
      local label = row.name
      local n = math.floor(avail / gp)
      if n < 8 then n = 8 end
      if ui.len(label) > n then label = ui.sub(label, n) end
      ui.caption(x + pmin, ry, label, fs(9.0), pal.fg, false)
      ui.surface(bl, ry + s(14), bw, s(4), s(2), pal.hover)
      local fw = bw * row.frac
      if fw > 0.5 then
        ui.surface(bl, ry + s(14), fw, s(4), s(2), pal.acc)
      end
    end
    i = i + 1
  end
end
