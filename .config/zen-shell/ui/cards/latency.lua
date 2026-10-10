-- latency.lua — the Latency card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). The engine (Rust) is backend + renderer; this module
-- paints the card once per frame through `ctx.ui.*`, the exact same Cmd
-- components the scene emitted, so geometry is byte-identical. The geometry
-- this module reproduces was ui/cards/latency.ron (the live, zero-`Ink` scene),
-- itself the retired `draw_latency_card` body:
--   Header     glyph U+E9E4 fs12 fg2 at pad, title "Latency" semibold 11.5 at
--              pad+17, meta "1.1.1.1" (the probe host) mono right at pad
--   Plot       x 12, top 22+10 = 32, width w−24−58 (the 58 px column the hero
--              number owns), height h−32−12
--   Sparkline  the pre-normalized 0..1 probe series (a 120 ms peak floor,
--              empty until two probes land) as a 2 px info trace, max: 1.0
--   Idle line  the plot's vertical midpoint = h/2 + 10, fg3, only while the
--              series is empty (a `Divider` with `cy_pct: 0.5`)
--   Hero ms    right 12 at plot_mid + 7 = h/2 + 17, display-medium
--   "ms"       right 12 at plot_mid − 7 = h/2 + 3, caption fg3
--   Status     x 12 at h − 23 (the pill row's baseline), fg2
--   Pill       30x20 r 6 right 12 bottom 8: raised plate, 5.5 px-centered
--              refresh glyph (fg3, acc while its key is hovered), over a
--              keyed 3 px-out 36x26 click halo (`ui.hit`)
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h
  local p = s(12)

  if api.card_show_glyph then
    ui.text(x + p, y + s(8), api.glyph, fs(12), pal.fg2, true)
  end
  if api.card_show_title then
    ui.title(x + p + s(17), y + s(9), "Latency", fs(11.5), pal.fg)
  end
  if api.meta ~= "" then
    ui.monor(x + w - p, y + s(11), api.meta, fs(8.5), pal.fg3)
  end

  local plot_x = x + p
  local plot_y = y + s(32)
  local plot_w = w - p * 2 - s(58)
  local plot_h = h - s(32) - s(12)
  local mid = plot_y + plot_h * 0.5

  if api.idle then
    ui.line(plot_x, mid, plot_x + plot_w, mid, s(1), pal.fg3)
  else
    ui.spark(plot_x, plot_y, plot_w, plot_h, api.series, pal.info, s(2), 1.0)
  end

  ui.heror(x + w - p, mid + s(7), api.val, fs(20), api.val_ink)
  ui.captionr(x + w - p, mid - s(7), "ms", fs(7.5), pal.fg3, false)

  local bh = s(20)
  local by = y + h - bh - s(8)
  ui.caption(x + p, by + s(5), api.status, fs(8.5), pal.fg2, false)

  local hov = ctx.hover == 31970
  local bx = x + w - p - s(30)
  ui.surface(bx, by, s(30), bh, s(6), hov and pal.raised_hl or pal.raised)
  ui.textc(bx + s(15), by + s(5.5), api.rerun_glyph, fs(9), hov and pal.acc or pal.fg3, true)
  ui.hit(bx, by, s(30), bh, 31970)
  ui.hit(bx - s(3), by - s(3), s(36), s(26), 31970)
end