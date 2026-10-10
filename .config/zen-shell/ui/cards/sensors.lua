-- sensors.lua — the Sensors card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). Geometry of record is ui/cards/sensors.ron (the live,
-- zero-`Ink` scene): Header (accent gauge glyph at pad 12 + "Sensors" title),
-- then one row per exposed sensor — tilt always meaningful once any sensor
-- exists, compass / gyro gated on their hardware. Each row is a dim fg2 label
-- at x+16 · a right value pinned 16 px in from the card's right edge (compass
-- renders the octant gauge glyph + heading in accent — the value string arrives
-- with the glyph baked in; gyro is a mono °/s triplet). One centered dim
-- caption when nothing is exposed.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h

  -- header — accent glyph shifts the title 17 px right (when shown)
  local p = s(12)
  local tx = p
  if api.card_show_glyph then
    ui.text(x + p, y + s(8), "\239\155\135", fs(12.0), pal.acc, true)
    tx = p + s(17)
  end
  if api.card_show_title then
    ui.title(x + tx, y + s(9), "Sensors", fs(11.5), pal.fg)
  end

  if api.sensors_none then
    ui.textc(x + w / 2, y + h / 2, "no IIO sensors", fs(9.0), pal.fg3, false)
  end
  if api.sensors_tilt then
    ui.text(x + s(16), y + s(32), "tilt", fs(9.0), pal.fg2, false)
    ui.textr(x + w - s(16), y + s(32), api.sensor_tilt, fs(9.5), pal.fg, false)
  end
  if api.sensors_compass then
    ui.text(x + s(16), y + s(52), "compass", fs(9.0), pal.fg2, false)
    ui.textr(x + w - s(16), y + s(52), api.sensor_compass, fs(9.5), pal.acc, true)
  end
  if api.sensors_gyro then
    ui.text(x + s(16), y + s(72), "gyro \194\176/s", fs(9.0), pal.fg2, false)
    ui.monor(x + w - s(16), y + s(72), api.sensor_gyro, fs(9.0), pal.fg)
  end
end
