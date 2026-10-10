-- moon.lua — the Moon card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). The engine (Rust) is backend + renderer; this module
-- paints the card once per frame through `ctx.ui.*`, the exact same Cmd
-- components the scene emitted, so geometry is byte-identical. The geometry
-- this module reproduces was ui/cards/moon.ron (the live, zero-`Ink` scene),
-- itself the retired `draw_moon_card` body:
--   Header     glyph U+F186 fs12 fg2 at pad, title "Moon" semibold 11.5 at
--              pad+17, meta "{:.0}%" (illumination) mono right at pad
--   Disc       the `Moon` primitive (`ui.moon`) — a dark base disc with the
--              lit portion painted as terminator slivers over one frame's
--              phase sample, at (x+pad+4+d/2, y+28+body_h/2), diameter =
--              max(min(body_h, w−2·pad−90), 30) with body_h = h−62
--   Captions   the phase name fs10.5 fg, "{:.0}% lit" fs8.5 fg2, and the
--              days-to-full line fs8.5 fg3 — a right-hand column from the
--              disc's center (x = cx + d/2 + 12), drawn only while the card
--              has ≥ 60 px of room to its right
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h
  local p = s(12)

  if api.card_show_glyph then
    ui.text(x + p, y + s(8), api.glyph, fs(12), pal.fg2, true)
  end
  if api.card_show_title then
    ui.title(x + p + s(17), y + s(9), "Moon", fs(11.5), pal.fg)
  end
  if api.meta ~= "" then
    ui.monor(x + w - p, y + s(11), api.meta, fs(8.5), pal.fg3)
  end

  -- the disc's box is the card's own body arithmetic (min/max floor of 30)
  local body_top = s(24) + s(4)
  local body_h = h - body_top - s(34)
  local d = math.max(math.min(body_h, w - p * 2 - s(90)), s(30))
  local cx = x + p + d / 2 + s(4)
  local cy = y + body_top + body_h / 2

  ui.moon(cx, cy, d, api.phase)

  -- the side column rides the disc's center, gated on card room like the
  -- drawer's `max_w > s(60)`
  local tx = cx + d / 2 + s(12)
  if x + w - p - tx > s(60) then
    ui.text(tx, cy - s(12), api.name, fs(10.5), pal.fg, false)
    ui.caption(tx, cy + s(3), api.lit, fs(8.5), pal.fg2, false)
    ui.caption(tx, cy + s(14), api.sub, fs(8.5), pal.fg3, false)
  end
end