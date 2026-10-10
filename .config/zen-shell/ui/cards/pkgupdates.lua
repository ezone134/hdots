-- pkgupdates.lua — the Packages card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). Geometry of record is ui/cards/pkgupdates.ron (the
-- live, zero-`Ink` scene): a hero count figure centred on the card midline,
-- exclusively gated — pending → DOWN glyph (22) at −18 in accent + accent
-- count hero (20, display/medium) at +10 + "updates" caption 9 fg2 at +34;
-- ok → CHECK glyph (22) at −18 in ok green + fg count hero + "up to date".
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h
  local cx, cy = x + w / 2, y + h / 2

  if api.pkg_pending then
    ui.textc(cx, cy + s(-18), "\239\129\163", fs(22.0), pal.acc, true)
    ui.heroc(cx, cy + s(10), api.pkg_count, fs(20.0), pal.acc)
    ui.textc(cx, cy + s(34), "updates", fs(9.0), pal.fg2, false)
  end
  if api.pkg_ok then
    ui.textc(cx, cy + s(-18), "\239\128\140", fs(22.0), pal.ok, true)
    ui.heroc(cx, cy + s(10), api.pkg_count, fs(20.0), pal.fg)
    ui.textc(cx, cy + s(34), "up to date", fs(9.0), pal.fg2, false)
  end
end
