-- sshvpn.lua — the SSH / VPN card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). Geometry of record is ui/cards/sshvpn.ron (the live,
-- zero-`Ink` scene): Header (pad 14) + a static list of connections from
-- `sshvpn_rows` — globe glyph (accent, icon font) for VPN lines, shield for
-- SSH, then the line at +30 in fg. Empty state via `{sshvpn_status}` (non-empty
-- template → the centered command is ALWAYS emitted). No click / scroll.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h

  local p = s(14)
  if api.card_show_title then
    ui.title(x + p, y + s(9), "SSH / VPN", fs(11.5), pal.fg)
  end

  ui.textc(x + w / 2, y + h / 2, api.sshvpn_status, fs(9.5), pal.fg, false)

  -- rows — first visible row flush under the header at y+32, 22 px stride
  local n = #api.rows
  if n == 0 then
    return
  end
  local row_h = s(22)
  local pitch = s(22)
  for i = 1, n do
    local ry = y + s(32) + (i - 1) * pitch
    if ry + row_h > y + h + 0.5 then break end
    local row = api.rows[i]
    ui.text(x + p + s(0), ry + s(1), row.icon, fs(10.0), pal.acc, true)
    ui.text(x + p + s(30), ry + s(1), row.line, fs(9.5), pal.fg, false)
  end
end
