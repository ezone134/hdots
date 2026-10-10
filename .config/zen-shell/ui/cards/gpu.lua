-- gpu.lua — the GPU card's entire UI layer (LuaJIT host, QUICKSHELL_MODEL
-- §Lua). The engine (Rust) is backend + renderer; this module paints the card
-- once per frame through `ctx.ui.*`, the exact same Cmd components the scene
-- emitted, so geometry is byte-identical. The geometry this module reproduces
-- was ui/cards/gpu.ron (the live zero-`Ink` scene), itself the retired
-- `draw_gpu_card` body:
--   Header    glyph U+F013 fs12 fg2 at pad 12, title "GPU" semibold fs11.5
--             at pad+17
--   Empty     "no GPU source" fg3 fs9, centered 6 above the middle, gated on
--             `gpu_empty`
--   Live      the util %%%% hero (display medium fs22 acc, right-anchored at
--             card_w−pad), the "utilization" caption fs8.5 fg2, the 5 px util
--             bar 26 below the header; the U+F2C9 thermal glyph + mono "{:.0}
--             °C" row, and — only when `gpu_vram_live` — the VRAM mono line
--             and 4 px info bar
-- The parity test `the_gpu_lua_module_paints_exactly_like_the_scene`
-- (src/lua/mod.rs) asserts this module produces IDENTICAL commands for every
-- state; once it holds and this module is live, gpu.ron is DELETED.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

local ICON_GPU = "\239\128\147"     -- U+F013
local ICON_THERMAL = "\239\139\137" -- U+F2C9

function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h
  local pad = s(12)
  local hdr = s(24)

  -- header — the cog glyph shifts the title right by 17
  local tx = pad
  if api.card_show_glyph then
    ui.text(x + pad, y + s(8), ICON_GPU, fs(12), pal.fg2, true)
    tx = pad + s(17)
  end
  if api.card_show_title then
    ui.title(x + tx, y + s(9), "GPU", fs(11.5), pal.fg)
  end

  if api.empty then
    ui.captionc(x + w / 2, y + h / 2 - s(6), "no GPU source", fs(9), pal.fg3, false)
    return
  end

  -- util big number + bar
  ui.heror(x + w - pad, y + hdr, api.pct, fs(22), pal.acc)
  ui.caption(x + pad, y + hdr + s(4), "utilization", fs(8.5), pal.fg2, false)
  local bw = w - pad * 2
  ui.bar(x + pad, y + hdr + s(26), bw, s(5), s(2.5), tonumber(api.frac), 1.0,
    pal.acc, pal.hover, false)

  -- temp + vram rows
  local ry = y + hdr + s(38)
  ui.text(x + pad, ry, ICON_THERMAL, fs(9), pal.fg2, true)
  ui.mono(x + pad + s(14), ry + s(1), api.temp, fs(9.5), pal.fg)
  if api.vram_live then
    ui.mono(x + pad, ry + s(15), api.vram_txt, fs(9.5), pal.fg)
    ui.bar(x + pad, ry + s(27), bw, s(4), s(2.0), tonumber(api.vram_frac), 1.0,
      pal.info, pal.hover, false)
  end
end
