-- smarthealth.lua — the Disk health card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). The engine (Rust) is backend + renderer; this module
-- paints the card once per frame through `ctx.ui.*` — the exact same Cmd
-- components the scene emits, so geometry is byte-identical. Geometry of record
-- was ui/cards/smarthealth.ron (the live, zero-`Ink` scene):
--   Header  title "Disk health" semibold 11.5, glyph `{sm_icon}` fg2, meta
--           `{sm_meta}` ("{n} disk(s)") caption fg3 right at pad 12
--   Rows    sm_rows: one row per disk from y+28, 23 px stride — mono dev name
--           (danger on failure), dim model caption at +40 truncated 96 px, mono
--           temp right at 56 (warn ≥ 55°), ✓/✕ status glyph right at 10
--   Text    "no SMART disks" caption fg3 centered (only while empty)
--   Hit     bottom-right refresh well (key 31980), raised → raised_hl, fg3 → acc
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine fs()).

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h
  local p = s(12)

  local tx = p
  if api.card_show_glyph then
    ui.text(x + p, y + s(8), api.sm_icon, fs(12), pal.fg2, true)
    tx = p + s(17)
  end
  if api.card_show_title then
    ui.title(x + tx, y + s(9), "Disk health", fs(11.5), pal.fg)
  end
  if api.sm_meta ~= "" then
    ui.captionr(x + w - p, y + s(11), api.sm_meta, fs(8.5), pal.fg3, false)
  end

  if not api.sm_live then
    ui.textc(x + w / 2, y + h / 2, "no SMART disks", fs(9), pal.fg3, false)
  else
    local row_h = s(23)
    local pitch = s(23)
    local avail = w - p * 2 - s(96)
    local gp = 0.62 * fs(8.5)
    if gp < 1 then gp = 1 end
    local i = 0
    while i < #api.rows do
      local ry = y + s(28) + i * pitch
      if ry + row_h > y + h - s(28) + 0.5 then break end
      local row = api.rows[i + 1]
      ui.mono(x + p, ry, row.dev, fs(8.5), row.dcolor)
      local m = row.model
      local n = math.floor(avail / gp)
      if n < 8 then n = 8 end
      if ui.len(m) > n then m = ui.sub(m, n) end
      ui.caption(x + p + s(40), ry, m, fs(8.5), row.mcolor, false)
      ui.monor(x + w - s(56), ry, row.temp, fs(8.5), row.tcolor)
      ui.captionr(x + w - s(10), ry + s(-0.5), row.sg, fs(10), row.scolor, true)
      i = i + 1
    end
  end

  local key = 31980
  local hov = (ctx.hover == key)
  local bx = x + w - s(45)
  local by = y + s(29.5)
  local bw = s(30)
  local bh = s(20)
  ui.surface(bx, by, bw, bh, s(6), hov and pal.raised_hl or pal.raised)
  ui.textc(bx + bw / 2, by, "\239\128\161", fs(9), hov and pal.acc or pal.fg3, true)
  ui.hit(bx, by, bw, bh, key)
end
