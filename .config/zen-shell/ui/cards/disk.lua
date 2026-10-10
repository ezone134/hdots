-- disk.lua — the Disk card's entire UI layer (LuaJIT host, QUICKSHELL_MODEL
-- §Lua). The engine (Rust) is backend + renderer; this module paints the card
-- once per frame through `ctx.ui.*`, the exact same Cmd components the scene
-- emits, so geometry is byte-identical. Geometry of record is ui/cards/disk.ron
-- (the live, zero-`Ink` scene):
--   Header  title "Disk" semibold 11.5, meta `{disk_meta}` caption fg3 right at pad
--   Rows    disk_rows: rows from y+32, 24 px stride, 14 px pad — mount label
--           fg2 caption at the pad, mono `used/total GB pct%` right at the pad,
--           a 6 px meter bar 13 px under the row top (r 3) whose fill takes the
--           shared warn/crit ladder (75 / 90)
--   Text    "No partitions" caption fg3 centered, 6 px above the card middle
--           (only while the list is empty)
-- The parity test `the_disk_lua_module_paints_exactly_like_the_scene`
-- (src/lua/mod.rs) asserts this module produces IDENTICAL commands for every
-- state; once it holds and this module is live, disk.ron is DELETED.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h

  -- header — title + caption meta (the meta disappears when empty)
  local p = s(14)
  if api.card_show_title then
    ui.title(x + p, y + s(9), "Disk", fs(11.5), pal.fg)
  end
  if api.disk_meta ~= "" then
    ui.captionr(x + w - p, y + s(11), api.disk_meta, fs(8.5), pal.fg3, false)
  end

  if api.disk_live then
    -- one row per mount: label · right mono readout · full-width meter bar
    local row_h = s(24)
    local bar_w = w - p * 2
    local i = 0
    while i < #api.rows do
      local ry = y + s(32) + i * row_h
      if ry + row_h > y + h - s(4) then break end
      local row = api.rows[i + 1]
      ui.caption(x + p, ry, row.label, fs(9.0), pal.fg2, false)
      ui.monor(x + w - p, ry, row.value, fs(9.0), pal.fg)
      local bar_y = ry + s(13)
      ui.surface(x + p, bar_y, bar_w, s(6), s(3), pal.hover)
      local fill_w = bar_w * (row.pct / 100.0)
      if fill_w > 0.5 then
        ui.surface(x + p, bar_y, fill_w, s(6), s(3), row.color)
      end
      i = i + 1
    end
  else
    ui.captionc(x + w / 2, y + h / 2 - s(6), "No partitions", fs(9.0), pal.fg3, false)
  end
end
