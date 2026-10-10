-- gauges.lua — the Memory/Disk bead-ring card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). The engine (Rust) is backend + renderer; this module
-- paints the card once per frame through `ctx.ui.*`, the exact same Cmd
-- components the scene emits, so geometry is byte-identical. Geometry of
-- record is ui/cards/gauges.ron (the live, zero-`Ink` scene):
--   two columns (w ≥ 170) at x + w·0.27 / 0.73, else one centered column
--   ring r 30 (h ≥ 116) else 22 · dot 5 / 4 · cy = card centre
--   hero % at cy − 8 (13px big, 11px small), label at cy + r + 15 and the
--   capacity line at cy + r + 28 — the last two only when h ≥ 94
-- The parity test `the_gauges_lua_module_paints_exactly_like_the_scene`
-- (src/lua/mod.rs) asserts this module produces IDENTICAL commands for every
-- state; once it holds and this module is live, gauges.ron is DELETED.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

-- ── one gauge column: bead ring + hero percent + label/capacity ─────────────
local function gauge(ctx, cx, pct_txt, label, gb, lit, fill)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local y, h = ctx.y, ctx.h
  local api = ctx.api
  local big = h >= s(116)
  local r = s(big and 30 or 22)
  local dot = s(big and 5 or 4)
  local cy = y + h / 2
  ui.ring(cx, cy, r, dot, 36, lit, fill, api.dim)
  ui.heroc(cx, cy - s(8), pct_txt, fs(big and 13 or 11), pal.fg)
  if h >= s(94) then
    ui.captionc(cx, cy + s(big and 45 or 37), label, fs(10), pal.fg, false)
    ui.monoc(cx, cy + s(big and 58 or 50), gb, fs(8), pal.fg2)
  end
end

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s = ctx.s
  local api = ctx.api
  local x, w = ctx.x, ctx.w

  if w >= s(170) then
    gauge(ctx, x + w * 0.27, api.mem_pct_txt, "Memory", api.mem_gb, api.mem_lit, api.mem_fill)
    gauge(ctx, x + w * 0.73, api.disk_pct_txt, "Disk", api.disk_gb, api.disk_lit, api.disk_fill)
  else
    gauge(ctx, x + w / 2, api.mem_pct_txt, "Memory", api.mem_gb, api.mem_lit, api.mem_fill)
  end
end
