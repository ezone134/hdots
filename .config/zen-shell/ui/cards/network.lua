-- network.lua — the Network card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). Geometry of record is network.ron (the live,
-- zero-`Ink` scene):
--   Header     title "Network" semibold 11.5 pad 12, meta {net_meta} mono
--              fg3 right (empty until two history samples)
--   When(59)   dual normalized history graph: download info, upload acc,
--              plot x 12 y 28, inset 12 left/right, 21 above the bottom —
--              the `Spark` line primitive
--   Row(bottom) centered speed line at h − 20, padded 12: two 79 px groups
--              (↓ glyph + mono value) with a 26 px gap between them
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h
  local p = s(12)

  if api.card_show_title then
    ui.title(x + p, y + s(9), "Network", fs(11.5), pal.fg)
  end
  if api.meta ~= "" then
    ui.monor(x + w - p, y + s(11), api.meta, fs(8.5), pal.fg3)
  end

  if h >= s(59) then
    local sx = x + s(12)
    local sy = y + s(28)
    local sw = w - s(24)
    local sh = h - s(49)
    ui.spark(sx, sy, sw, sh, api.down_data, pal.info, s(2))
    ui.spark(sx, sy, sw, sh, api.up_data, pal.acc, s(2))
  end

  local cy = y + h - s(20) + s(12)
  local glyph_y = cy + s(20) / 2
  local cx = x + s(12)
  ui.textc(cx + s(9) / 2, glyph_y, "\239\129\163", fs(9), pal.info, true)
  ui.mono(cx + s(11), glyph_y, api.net_down, fs(10), pal.info)
  cx = cx + s(79) + s(26)
  ui.textc(cx + s(9) / 2, glyph_y, "\239\129\162", fs(9), pal.acc, true)
  ui.mono(cx + s(11), glyph_y, api.net_up, fs(10), pal.acc)
end