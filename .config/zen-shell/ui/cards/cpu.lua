-- cpu.lua — the CPU card's entire UI layer (LuaJIT host, QUICKSHELL_MODEL
-- §Lua). The engine (Rust) is backend + renderer; this module paints the card
-- once per frame through `ctx.ui.*`, the exact same Cmd components the scene
-- emits, so geometry is byte-identical. Geometry of record is ui/cards/cpu.ron
-- (the live, zero-`Ink` scene):
--   Header  title "CPU" semibold 11.5, meta `{cpu_meta}` mono fg3 right at pad
--   Ring    cpu_ring: (38, cy+13) r 26 dot 4, 36 beads, gauge ink, hover track
--   Text    `{cpu}%` semibold 15 in the gauge ink, centered on its box
--   Rows    cpu_rows: load / freq / temp (label fg2, mono value right at pad)
-- The parity test `the_cpu_lua_module_paints_exactly_like_the_scene`
-- (src/lua/mod.rs) asserts this module produces IDENTICAL commands for every
-- state; once it holds and this module is live, cpu.ron is DELETED.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h

  -- header — title + mono meta (the meta disappears when unbound/empty)
  local p = s(14)
  if api.card_show_title then
    ui.title(x + p, y + s(9), "CPU", fs(11.5), pal.fg)
  end
  if api.cpu_meta ~= "" then
    ui.monor(x + w - p, y + s(11), api.cpu_meta, fs(8.5), pal.fg3)
  end

  -- bead-ring gauge, centered in its column
  ui.ring(x + s(38), y + h / 2 + s(13), s(26), s(4), 36, api.cpu_lit, api.cpu_gauge_col, pal.hover)

  -- hero percent — a top-level `Text` with `center`, so it centers on the
  -- box that spans from its x to the card's right edge
  local sx = x + s(38)
  ui.titlec(sx + (w - s(38)) / 2, y + h / 2 + s(6), api.cpu .. "%", fs(15), api.cpu_gauge_col)

  -- metric rows (load / freq / temp); values right-aligned at the card pad
  local ry = y + s(30)
  for i = 1, #api.rows do
    if ry + s(22) > y + h + 0.5 then break end
    local row = api.rows[i]
    ui.caption(x + s(14) + s(62), ry, row.label, fs(8.5), pal.fg2, false)
    if row.value ~= "" then
      ui.monor(x + w - s(14), ry, row.value, fs(8.5), row.color)
    end
    ry = ry + s(22)
  end
end
