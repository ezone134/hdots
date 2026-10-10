-- cpugpu.lua — the CPU/GPU card's entire UI layer (LuaJIT host,
-- QUICKSHELL_MODEL §Lua). The engine (Rust) is backend + renderer; this module
-- paints the card once per frame through `ctx.ui.*`, the exact same Cmd
-- components the scene emitted, so geometry is byte-identical. The geometry
-- this module reproduces was ui/cards/cpugpu.ron (the live, zero-`Ink` scene),
-- itself the retired `draw_cpugpu_card` body:
--   When w>=170  the legend rows top-right: an 8x8 r2 colour dot + a
--                right-anchored % at pad 14 (CPU dot y+18 / text y+15, GPU dot
--                y+39 / text y+36). The GPU dot shows only while a source
--                exists; the "GPU n/a" caption takes its place otherwise.
--   When wide+h  the two 0..1 histories as traces, GPU UNDER CPU, plot box
--                14 in from each side, from y+52, stopping 14 above the
--                bottom (needs h >= 90: 52+14+24).
--   When narrow  the same plot full-bleed from y+12 (needs h >= 50).
-- The trace `max: 1.0` matters: the histories are raw 0..1 fractions, so an
-- explicit max reproduces the drawer's own `clamp(0,1)` instead of
-- peak-normalizing a quiet CPU.
-- The parity test `the_cpugpu_lua_module_paints_exactly_like_the_scene`
-- (src/lua/mod.rs) asserted this module produced IDENTICAL commands for every
-- state; once it held, this module went live and cpugpu.ron was DELETED.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h

  -- the legend rows live only on a wide card (the drawer's `w >= s(170)`):
  -- colour dot + right-anchored % at pad 14, CPU above GPU; the GPU dot only
  -- while a source exists (else just the "GPU n/a" caption)
  if w >= s(170) then
    local rx = x + w - s(14)
    ui.surface(rx - s(52), y + s(18), s(8), s(8), s(2), api.cpu_ink)
    ui.textr(rx, y + s(15), api.cpu_txt, fs(10.0), pal.fg, false)
    if api.gpu_live then
      ui.surface(rx - s(52), y + s(39), s(8), s(8), s(2), api.gpu_ink)
    end
    ui.textr(rx, y + s(36), api.gpu_txt, fs(10.0), pal.fg, false)
  end

  -- the shared plot box: 14 px in from each side. The wide plot sits 52 px
  -- down and needs 90 px of card (52+14+24); the narrow one starts at 12 px
  -- and needs 50 (12+14+24) — the drawer's own `plot_h < s(24)` floor folded
  -- into the `When` gates
  local plot_y
  if w >= s(170) then
    if h >= s(90) then plot_y = y + s(52) end
  elseif h >= s(50) then
    plot_y = y + s(12)
  end
  if plot_y then
    local plot_x = x + s(14)
    local plot_w = w - s(28)
    local plot_h = y + h - plot_y - s(14)
    -- the two 0..1 traces: GPU under CPU; the GPU hides while no source
    -- exists (its history fills with zeros, and a flat line at the plot's
    -- floor is noise)
    if api.gpu_data then
      ui.spark(plot_x, plot_y, plot_w, plot_h, api.gpu_data, api.gpu_ink, s(2), 1.0)
    end
    if api.cpu_data then
      ui.spark(plot_x, plot_y, plot_w, plot_h, api.cpu_data, api.cpu_ink, s(2), 1.0)
    end
  end
end
