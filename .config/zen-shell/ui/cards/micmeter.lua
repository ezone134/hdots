-- micmeter.lua — the Mic card's entire UI layer (LuaJIT host, QUICKSHELL_MODEL
-- §Lua). Geometry of record is micmeter.ron (the live, zero-`Ink` scene):
--   Header     glyph {mic_icon} fs12 fg2, title "Mic" semibold 11.5 at +17,
--              meta {mic_meta} mono fg3 right, pad 12
--   Bar        level meter: x 12 y 34 h 8 r 4, both-side 12 inset — track
--              `hover`, fill the clipped-to-red ink, value already 0..1
--   Text       {mic_status} Ui/Regular 9 at x 12 y 50 in the status ink
--   Surface    22 px mute button bottom-anchored 12 px above the card bottom
--              (both-side 12 inset), hover-aware raised fill
--   Text       {mic_btn_label} centered at the button top + 6, icon voice
--   Hit        the button's 3 px click halo, key MIC_MUTE_KEY
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h
  local p = s(12)

  if api.card_show_glyph then
    ui.text(x + p, y + s(8), api.icon, fs(12), pal.fg2, true)
  end
  if api.card_show_title then
    ui.title(x + p + s(17), y + s(9), "Mic", fs(11.5), pal.fg)
  end
  if api.meta ~= "" then
    ui.monor(x + w - p, y + s(11), api.meta, fs(8.5), pal.fg3)
  end

  local bar_w = math.max(w - p * 2, 2.0)
  ui.bar(x + s(12), y + s(34), bar_w, s(8), s(4), api.lvl, 1.0, api.bar_ink, pal.hover, false)
  ui.caption(x + s(12), y + s(50), api.status, fs(9), api.status_ink, false)

  ui.surface(x + s(12), y + h - s(12) - s(22), w - s(24), s(22), s(6), api.btn_fill)
  ui.textc(x + w / 2, y + h - s(28), api.btn_label, fs(9), api.btn_ink, true)
  ui.hit(x + s(9), y + h - s(37), w - s(18), s(28), 31910)
end