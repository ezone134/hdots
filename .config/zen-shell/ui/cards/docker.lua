-- docker.lua — the Docker card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). The engine (Rust) is backend + renderer; this module
-- paints the card once per frame through `ctx.ui.*` — the exact same Cmd
-- components the scene emits, so geometry is byte-identical. Geometry of record
-- was ui/cards/docker.ron (the live, zero-`Ink` scene):
--   Header  title "Docker" semibold 11.5 at pad 14 (no glyph, no meta)
--   Rows    docker_rows: one row per container from y+32, 26 px stride — a 6 px
--           status dot (+3, r 3) at the left edge inked by state (Up = ok green,
--           else danger), the name at +10 in fg, the image label right-anchored
--           14 in, capped at 20 chars
--   Text    "No containers" caption fg centered (only while empty)
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine fs()).

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h
  local p = s(14)

  if api.card_show_title then
    ui.title(x + p, y + s(9), "Docker", fs(11.5), pal.fg)
  end

  if #api.rows == 0 then
    ui.textc(x + w / 2, y + h / 2, "No containers", fs(9.5), pal.fg, false)
  else
    local row_h = s(26)
    local pitch = s(26)
    local i = 0
    while i < #api.rows do
      local ry = y + s(32) + i * pitch
      if ry + row_h > y + h - s(4) + 0.5 then break end
      local row = api.rows[i + 1]
      ui.surface(x + p, ry + s(3), s(6), s(6), s(3), row.dot)
      ui.caption(x + p + s(10), ry, row.name, fs(9), pal.fg, false)
      local img = row.image
      if ui.len(img) > 20 then img = ui.sub(img, 20) end
      ui.captionr(x + w - s(14), ry, img, fs(7.5), pal.fg, false)
      i = i + 1
    end
  end
end
