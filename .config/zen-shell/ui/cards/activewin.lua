-- activewin.lua — the Active Window card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). Geometry of record is ui/cards/activewin.ron (the
-- live, zero-`Ink` scene): a twin-state hero, everything centred on the card
-- midline. Empty → dim ACTIVE glyph (20) 14 px under the top + "No window"
-- caption centred (+10 off the middle); live → app class 13 at y+14 and a
-- 28-char title 9.5 at y+36; a per-app RAM pill `{aw_rss}` 9 fg2 centred 16 px
-- above the bottom, gated on `aw_has_rss`. `active_win_empty` / `active_win_has`
-- gate the two hero states.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h
  local cx = x + w / 2

  if api.active_win_empty then
    -- dim ACTIVE glyph over the centered "No window" caption
    ui.textc(cx, y + s(14), "\239\139\144", fs(20.0), pal.fg, true)
    ui.textc(cx, y + h / 2 + s(10), "No window", fs(10.0), pal.fg, false)
  end
  if api.active_win_has then
    ui.textc(cx, y + s(14), api.aw_class, fs(13.0), pal.fg, false)
    ui.textc(cx, y + s(36), api.aw_title, fs(9.5), pal.fg, false)
  end
  if api.aw_has_rss then
    ui.textc(cx, y + h - s(16), api.aw_rss, fs(9.0), pal.fg2, false)
  end
end
