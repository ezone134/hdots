-- diskio.lua — the Disk I/O card's entire UI layer (LuaJIT host, QUICKSHELL_MODEL
-- §Lua). The engine (Rust) is backend + renderer; this module paints the card
-- once per frame through `ctx.ui.*`, the exact same Cmd components the scene
-- emits, so geometry is byte-identical. Geometry of record is ui/cards/diskio.ron
-- (the live, zero-`Ink` scene):
--   Surface/Text  legend: R dot (info) + caption fg, W dot (acc) + caption fg,
--                 right-anchored at the exact Rust insets (dot right edge = card
--                 − 51, caption right edge = card − 14)
--   Header        title "Disk I/O" semibold 11.5, pad 12 (no glyph)
--   When(min_h 78) dual normalized history graph (read info first, write acc
--                 over it), plot box x 14 y 46, inset 14 left/right, 12 above
--                 the bottom — the `Spark` line primitive
-- Coordinates are DEVICE px (`ctx.s` / `ctx.fs` scale base px / font size).
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h

  ui.surface(x + w - s(58), y + s(16), s(7), s(7), s(2), pal.info)
  ui.textr(x + w - s(14), y + s(14), "R", fs(9), pal.fg, false)
  ui.surface(x + w - s(58), y + s(34), s(7), s(7), s(2), pal.acc)
  ui.textr(x + w - s(14), y + s(32), "W", fs(9), pal.fg, false)

  if api.card_show_title then
    ui.title(x + s(12), y + s(9), "Disk I/O", fs(11.5), pal.fg)
  end

  if h >= s(78) then
    local sx = x + s(14)
    local sy = y + s(46)
    local sw = w - s(28)
    local sh = h - s(58)
    ui.spark(sx, sy, sw, sh, api.r_data, pal.info, s(2))
    ui.spark(sx, sy, sw, sh, api.w_data, pal.acc, s(2))
  end
end
