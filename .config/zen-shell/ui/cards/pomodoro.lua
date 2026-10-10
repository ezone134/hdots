-- pomodoro.lua — the Focus (pomodoro) card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). The engine (Rust) is backend + renderer; this
-- module paints the card once per frame through `ctx.ui.*`, the exact same Cmd
-- components the scene emits, so geometry is byte-identical. Geometry of
-- record is ui/cards/pomodoro.ron (the live, zero-`Ink` scene):
--   Header  glyph (12, 8) fg2, title "Focus" @ (12+17, 9) 11.5, meta fg3 right
--   Text    `{pomo_hero}` ui/light 26, centered on the card, top y+30
--   Text    `{pomo_phase_lbl}` ui/regular 8.5 fg3, centered, top y+58
--   Bar     12 px insets, 4 tall, r 2, top y+72 — fill `pomo_bar_ink`, track hover
--   Hits    `[-]`/+ readout trio at y+82/y+85 (idle only) · Start/Pause + Reset,
--          52×22 chips at bottom−32, 6 px apart around the middle
-- The parity test `the_pomodoro_lua_module_paints_exactly_like_the_scene`
-- (src/lua/mod.rs) asserts this module produces IDENTICAL commands for every
-- state; once it holds and this module is live, pomodoro.ron is DELETED.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

local ICON_TIMER = "\239\149\179" -- U+F573 timer (the drawer's mark)

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h

  -- header — glyph shifts the title right by 17
  local p = s(12)
  local tx = p
  if api.card_show_glyph then
    ui.text(x + p, y + s(8), ICON_TIMER, fs(12.0), pal.fg2, true)
    tx = p + s(17)
  end
  if api.card_show_title then
    ui.title(x + tx, y + s(9), "Focus", fs(11.5), pal.fg)
  end
  if api.pomo_meta ~= "" then
    ui.captionr(x + w - p, y + s(11), api.pomo_meta, fs(8.5), pal.fg3, false)
  end

  -- hero mm:ss — ui/light, centered on the card's center line
  ui.textc(x + w / 2, y + s(30), api.pomo_hero, fs(26.0), api.pomo_ink, false)
  -- phase word — ui/regular, centered, under the numerals
  ui.captionc(x + w / 2, y + s(58), api.pomo_phase_lbl, fs(8.5), pal.fg3, false)
  -- progress bar: `w − 12 − 12` spans between the insets, r 2 = y/2 capsule
  ui.bar(x + s(12), y + s(72), w - s(12) - s(12), s(4), s(2),
         tonumber(api.pomo_frac), 1.0, api.pomo_bar_ink, pal.hover, false)

  -- duration steppers (idle only): `[-] 25m [+]` — 18×18 chips, r 6, on y+82,
  -- labels centered in the chips on y+85.5 (82 + label_dy 3.5)
  if api.pomo_idle then
    local bw = s(18)
    local sy = y + s(82)
    local ly = sy + s(3.5)
    local dec = x + w / 2 + s(api.pomo_dec_dx)
    local dcol = (ctx.hover == 30733) and pal.hover_hl or api.pomo_chip_fill
    ui.surface(dec, sy, bw, bw, s(6), dcol)
    ui.textc(dec + bw / 2, ly, "-", fs(9.5), pal.fg, false)
    ui.hit(dec, sy, bw, bw, 30733)
    ui.captionc(x + w / 2, y + s(85), api.pomo_focus_readout, fs(8.5), pal.fg3, false)
    local inc = x + w / 2 + s(api.pomo_inc_dx)
    local icol = (ctx.hover == 30732) and pal.hover_hl or api.pomo_chip_fill
    ui.surface(inc, sy, bw, bw, s(6), icol)
    ui.textc(inc + bw / 2, ly, "+", fs(9.5), pal.fg, false)
    ui.hit(inc, sy, bw, bw, 30732)
  end

  -- controls: start/pause (accent tint) + reset — 52×22 chips, r 6, bottom
  -- edge 10 px above the card bottom on a 10-px line (cy2 = y + h − 32), with
  -- the pair straddling the middle 6 px apart. label_dy 6.25 = (22 − 9.5) / 2
  local bw2 = s(52)
  local bh2 = s(22)
  local cy2 = y + h - s(32)
  local ly2 = cy2 + s(6.25)
  local tog = x + w / 2 + s(-58)
  local tcol = (ctx.hover == 30730) and api.pomo_tog_hover or api.pomo_tog_fill
  ui.surface(tog, cy2, bw2, bh2, s(6), tcol)
  ui.textc(tog + bw2 / 2, ly2, api.pomo_tog_lbl, fs(9.5), pal.acc, false)
  ui.hit(tog, cy2, bw2, bh2, 30730)
  local rst = x + w / 2 + s(6)
  local rcol = (ctx.hover == 30731) and pal.hover_hl or api.pomo_chip_fill
  ui.surface(rst, cy2, bw2, bh2, s(6), rcol)
  ui.textc(rst + bw2 / 2, ly2, "Reset", fs(9.5), pal.fg, false)
  ui.hit(rst, cy2, bw2, bh2, 30731)
end