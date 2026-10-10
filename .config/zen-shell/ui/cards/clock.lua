-- clock.lua — the Clock card's entire UI layer (LuaJIT host, QUICKSHELL_MODEL
-- §Lua). The engine (Rust) is backend + renderer; this module paints the card
-- once per frame through `ctx.ui.*`, the exact same Cmd components the scene
-- emits, so geometry is byte-identical. Geometry of record is
-- ui/cards/clock.ron (the live, zero-`Ink` renderer):
--   hero time  "{clock}" at card centre, y − 4, display/medium 30
--   date line  "{date}"  at card centre, y − 22, ui/light 10, fg2
-- The parity test `the_clock_lua_module_paints_exactly_like_the_scene`
-- (src/lua/mod.rs) asserts this module produces IDENTICAL commands for every
-- state; once it holds and this module is live, clock.ron is DELETED.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h

  if api.clock ~= nil then
    ui.heroc(x + w / 2, y + h / 2 - s(4), api.clock, fs(30.0), pal.fg)
  end
  if api.date ~= nil then
    ui.textc(x + w / 2, y + h / 2 - s(22), api.date, fs(10.0), pal.fg2, false)
  end
end
