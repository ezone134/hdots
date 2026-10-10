-- branding.lua — the Branding card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). The engine (Rust) is backend + renderer; this module
-- paints the card once per frame through `ctx.ui.*`, the exact same Cmd
-- components the scene emitted, so geometry is byte-identical. The geometry
-- this module reproduces was ui/cards/branding.ron (the live zero-`Ink` scene),
-- itself the retired `draw_branding_card` body:
--   Header   glyph U+F004 fs12 fg2 at pad 16 (Light), title "Brand" semibold
--            fs11.5 at pad+17, meta none
--   Mark     the aspect-fit brand glyph (`{brand_glyph}`), centered on the
--            card's mid-line at a per-frame top: size = min(avail_w /
--            (chars·0.62), avail_h)·0.95 floored at 10 in BASE px, top =
--            24 + (avail_h − size)/2 — both computed in `scene_values` and
--            published as `brand_size` / `brand_y`
--   Empty    "no brand glyph in $states2/d" (fg3 fs9), centered 6 up from the
--            card's middle, gated on `brand_empty`
-- The parity test `the_branding_lua_module_paints_exactly_like_the_scene`
-- (src/lua/mod.rs) asserts this module produces IDENTICAL commands for every
-- state; once it holds and this module is live, branding.ron is DELETED.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

local ICON_SPARKLE = "\239\128\132" -- U+F004

function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h
  local p = s(16)

  -- header — the sparkle glyph shifts the title right by 17
  local tx = p
  if api.card_show_glyph then
    ui.text(x + p, y + s(8), ICON_SPARKLE, fs(12), pal.fg2, true)
    tx = p + s(17)
  end
  if api.card_show_title then
    ui.title(x + tx, y + s(9), "Brand", fs(11.5), pal.fg)
  end

  if api.empty then
    ui.textc(x + w / 2, y + h / 2 - s(6), "no brand glyph in $states2/d",
      fs(9), pal.fg3, false)
    return
  end

  -- the contain-fit mark, centered on the card's mid-line
  ui.brandc(x + w / 2, y + s(api.y), api.glyph, fs(api.size), pal.fg)
end
