-- powerdraw.lua — the Power card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). The engine (Rust) is backend + renderer; this module
-- paints the card once per frame through `ctx.ui.*`, the exact same Cmd
-- components the scene emits, so geometry is byte-identical. Geometry of
-- record is ui/cards/powerdraw.ron (the live, zero-`Ink` scene):
--   Header  title "Power", glyph U+F0E7, pad 12 (no meta)
--   Text    `{pw_b}` / `{pw_g}` right at pad (y 32 / 50), 10px fg
--   Surface the B / G dots: 7×7 r2, right edge 81 in from the card, y 34 / 52
--   Divider the flat idle trace (one sample): plot box mid-line, fg3, 1px
--   Spark   pw_b_series (accent) then pw_g_series (info), 2px line, max 1.0
--   Text    the "no power sensors" empty caption, centered a caption above mid
-- The parity test `the_powerdraw_lua_module_paints_exactly_like_the_scene`
-- (src/lua/mod.rs) asserts this module produces IDENTICAL commands for every
-- state; once it holds and this module is live, powerdraw.ron is DELETED.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

local ICON_BOLT = "\239\131\167" -- U+F0E7

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h

  -- header — the bolt glyph shifts the title right by 17
  local p = s(12)
  local tx = p
  if api.card_show_glyph then
    ui.text(x + p, y + s(8), ICON_BOLT, fs(12.0), pal.fg2, true)
    tx = p + s(17)
  end
  if api.card_show_title then
    ui.title(x + tx, y + s(9), "Power", fs(11.5), pal.fg)
  end

  -- the right column — watts right-aligned at the pad, the dot 88 in from the
  -- card's right edge (battery accent, GPU info blue)
  if api.pw_live then
    ui.textr(x + w - p, y + s(32), api.pw_b, fs(10.0), pal.fg, false)
    ui.textr(x + w - p, y + s(50), api.pw_g, fs(10.0), pal.fg, false)
    ui.surface(x + w - s(88), y + s(34), s(7), s(7), s(2), pal.acc)
    ui.surface(x + w - s(88), y + s(52), s(7), s(7), s(2), pal.info)
  end

  -- the plot box: 14 in from each side, the right column reserves 80 px more
  local plot_x = x + s(14)
  local plot_y = y + s(30)
  local plot_w = w - s(108)
  if plot_w < s(20) then plot_w = s(20) end
  local plot_h = h - s(42)
  if plot_h < s(10) then plot_h = s(10) end

  -- one sample cannot make a trace — the flat midline at the box's midpoint
  if api.pw_idle then
    local yb = plot_y + plot_h * 0.5
    ui.line(plot_x, yb, plot_x + plot_w, yb, s(1), pal.fg3)
  end

  -- the two traces (pre-normalized against the shared peak, so max 1.0)
  if api.pw_b_series then
    ui.spark(plot_x, plot_y, plot_w, plot_h, api.pw_b_series, pal.acc, s(2), 1.0)
  end
  if api.pw_g_series then
    ui.spark(plot_x, plot_y, plot_w, plot_h, api.pw_g_series, pal.info, s(2), 1.0)
  end

  -- the empty state, centered a caption's 6 px above the middle
  if api.pw_empty then
    ui.captionc(x + w / 2, y + h / 2 - s(6), "no power sensors", fs(9.0), pal.fg3, false)
  end
end
