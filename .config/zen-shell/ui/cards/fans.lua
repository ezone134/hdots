-- fans.lua — the Fans card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). Geometry of record was ui/cards/fans.ron (the live,
-- zero-`Ink` scene):
--   Header  title "Fans" semibold 11.5, glyph U+E9E4 fg2, meta "{n} fans" fg3
--           right at pad 12
--   Text    "no hwmon fan sensors" caption fg3 centered (gated on fans_empty)
--   Rows    fans_rows from y+28 on a 24 px pitch: dim fg2 label at pad 12
--           truncated 80 px · right mono "{rpm} rpm" at edge 12 (fg live / fg3
--           stopped) · a per-fan meter bar (3 px tall, top 13, r 1.5) whose
--           fraction is rpm / decayed peak, the fill fg over a hover track.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px, `ctx.fs(n)` scales a
-- base font size (rounded, matching the engine fs()).

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h
  local p = s(12)

  if api.card_show_glyph then
    ui.text(x + p, y + s(8), api.glyph, fs(12), pal.fg2, true)
  end
  if api.card_show_title then
    local tx = api.card_show_glyph and (p + s(17)) or p
    ui.title(x + tx, y + s(9), "Fans", fs(11.5), pal.fg)
  end
  if api.meta ~= "" then
    ui.captionr(x + w - p, y + s(11), api.meta, fs(8.5), pal.fg3, false)
  end

  if api.fans_empty then
    ui.textc(x + w / 2, y + h / 2, "no hwmon fan sensors", fs(9), pal.fg3, false)
  end

  if api.fans_live then
    local row_h = s(24)
    local pitch = s(24)
    local row_w = w - p * 2
    local avail = w - p * 2 - s(80)
    local gp = 0.62 * fs(8.5)
    if gp < 1 then gp = 1 end
    local i = 0
    while i < #api.rows do
      local ry = y + s(28) + i * pitch
      if ry + row_h <= y + h + 0.5 then
        local row = api.rows[i + 1]
        local label = row.label
        local n = math.floor(avail / gp)
        if n < 8 then n = 8 end
        if ui.len(label) > n then label = ui.sub(label, n) end
        ui.caption(x + p, ry, label, fs(8.5), row.lcolor, false)
        ui.monor(x + w - s(12), ry, row.rpm, fs(8.5), row.rcolor)
        ui.surface(x + p, ry + s(13), row_w, s(3), s(1.5), pal.hover)
        local fw = row_w * row.frac
        if fw > 0.5 then
          ui.surface(x + p, ry + s(13), fw, s(3), s(1.5), pal.fg)
        end
      end
      i = i + 1
    end
  end
end
