-- kblayout.lua — the KbLayout card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). The engine (Rust) is backend + renderer; this module
-- paints the card once per frame through `ctx.ui.*`, the exact same Cmd
-- components the scene emits, so geometry is byte-identical. Geometry of
-- record is ui/cards/kblayout.ron (the live, zero-`Ink` scene):
--   Text  keyboard glyph, ui/light 22, fg2, centered on the card, y − 12
--   Text  `{kblayout}`, display/medium 14, fg, centered, y + 15
-- The parity test `the_kblayout_lua_module_paints_exactly_like_the_scene`
-- (src/lua/mod.rs) asserts this module produces IDENTICAL commands for every
-- state; once it holds and this module is live, kblayout.ron is DELETED.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

-- glyph (icon font code point, UTF-8 byte escape) — U+F11C keyboard
local ICON_KEYBOARD = "\239\132\156"

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local cx = ctx.x + ctx.w / 2
  local cy = ctx.y + ctx.h / 2

  -- quiet keyboard mark above, the layout label below as the hero figure
  ui.textc(cx, cy - s(12), ICON_KEYBOARD, fs(22.0), pal.fg2, true)
  ui.heroc(cx, cy + s(15), api.kb_layout, fs(14.0), pal.fg)
end
