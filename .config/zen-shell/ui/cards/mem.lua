-- mem.lua — the Memory card's entire UI layer (LuaJIT host, QUICKSHELL_MODEL
-- §Lua). The engine (Rust) is backend + renderer; this module paints the card
-- once per frame through `ctx.ui.*`, the exact same Cmd components the scene
-- emits, so geometry is byte-identical. Geometry of record is ui/cards/mem.ron
-- (the live, zero-`Ink` scene):
--   Header  title "Memory" semibold 11.5 at pad
--   Ring    mem_pct_f: (38, cy+13) r 26 dot 4, 36 beads, accent ink, hover track
--   Text    `{mem}%` semibold 15 fg, centered on its box
--   Rows    mem_rows: total / available / cached / free (label fg2, mono value)
--   Bar     swap bar along the bottom (accent over hover), while swap is in use
-- The parity test `the_mem_lua_module_paints_exactly_like_the_scene`
-- (src/lua/mod.rs) asserts this module produces IDENTICAL commands for every
-- state; once it holds and this module is live, mem.ron is DELETED.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h

  -- header — title only
  local p = s(14)
  if api.card_show_title then
    ui.title(x + p, y + s(9), "Memory", fs(11.5), pal.fg)
  end

  -- bead-ring gauge, centered in its column (accent ink)
  ui.ring(x + s(38), y + h / 2 + s(13), s(26), s(4), 36, api.mem_lit, pal.acc, pal.hover)

  -- hero percent — centered on the box from its x to the card's right edge
  local sx = x + s(38)
  ui.titlec(sx + (w - s(38)) / 2, y + h / 2 + s(6), api.mem .. "%", fs(15), pal.fg)

  -- metric rows (total / available / cached / free); mono value at the pad
  local ry = y + s(30)
  for i = 1, #api.rows do
    if ry + s(22) > y + h + 0.5 then break end
    local row = api.rows[i]
    ui.caption(x + s(14) + s(62), ry, row.label, fs(8.5), pal.fg2, false)
    if row.value ~= "" then
      ui.monor(x + w - s(14), ry, row.value, fs(8.5), pal.fg)
    end
    ry = ry + s(22)
  end

  -- swap bar along the bottom (present only while swap is in use)
  if api.swap_show then
    local bsx = x + s(14)
    local bsy = y + s(124)
    local bsw = (w - s(28))
    if bsw < 2.0 then bsw = 2.0 end
    local bsh = s(5)
    ui.surface(bsx, bsy, bsw, bsh, s(2.5), pal.hover)
    local frac = api.swap_pct / 100
    if frac < 0.0 then frac = 0.0 elseif frac > 1.0 then frac = 1.0 end
    if frac > 0.001 then
      ui.surface(bsx, bsy, bsw * frac, bsh, s(2.5), pal.acc)
    end
  end
end
