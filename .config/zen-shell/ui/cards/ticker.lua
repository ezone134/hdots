-- ticker.lua — the Prices card's entire UI layer (LuaJIT host, QUICKSHELL_MODEL
-- §Lua). The engine (Rust) is backend + renderer; this module paints the card
-- once per frame through `ctx.ui.*`, the exact same Cmd components the scene
-- emits, so geometry is byte-identical. Geometry of record is
-- ui/cards/ticker.ron (the live, zero-`Ink` scene):
--   Header  glyph U+F201 fg2 12 @ (pad, 8); title "Prices" semibold 11.5
--           @ (pad + 17, 9); meta "crypto" caption fg3 right @ (w − pad, 11)
--   Text    `{ticker_status}` ui/light 9 fg3 centered at (w/2, h/2 − 6) — the
--           template is non-empty, so the command is ALWAYS emitted (empty
--           string once prices exist)
--   Rows    ticker_rows: first visible row at y+26, 18 px stride, 12 px pad —
--           sym ui/light 9.5 fg at +6, price ui/light 9.5 fg2 at +62, chg%
--           ui/light 9 right-anchored 12 px in (ok green / danger red); the
--           hovered row paints a hover_hl band (r 5) and every drawn row
--           registers `key_base + visible index` (14600 + i)
-- The parity test `the_ticker_lua_module_paints_exactly_like_the_scene`
-- (src/lua/mod.rs) asserts this module produces IDENTICAL commands for every
-- state; once it holds and this module is live, ticker.ron is DELETED.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

-- glyph (icon font code point, UTF-8 byte escape) — U+F201 line-chart
local ICON_TICKER = "\239\136\129"

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api
  local x, y, w, h = ctx.x, ctx.y, ctx.w, ctx.h

  -- header — glyph shifts the title right by 17; meta pinned right
  local p = s(12)
  if api.card_show_glyph then
    ui.text(x + p, y + s(8), ICON_TICKER, fs(12.0), pal.fg2, true)
  end
  if api.card_show_title then
    ui.title(x + p + s(17), y + s(9), "Prices", fs(11.5), pal.fg)
  end
  if api.meta ~= "" then
    ui.captionr(x + w - p, y + s(11), api.meta, fs(8.5), pal.fg3, false)
  end

  -- status line — always emitted (empty while prices are on screen)
  ui.textc(x + w / 2, y + h / 2 - s(6), api.ticker_status, fs(9.0), pal.fg3, false)

  -- rows — window starting at `scroll`, first visible row flush under the
  -- header at y+26; rows past the fold are dropped (the scene's +0.5 fudge)
  local n = #api.rows
  if n == 0 then
    return
  end
  local row_h = s(18)
  local pitch = s(18)
  local row_w = w - p * 2
  if row_w < 2 then row_w = 2 end
  local start = math.min(api.scroll, n - 1)
  for i = start, n - 1 do
    local j = i - start
    local ry = y + s(26) + j * pitch
    if ry + row_h > y + h + 0.5 then break end
    local key = api.key_base + j
    local row = api.rows[i + 1]
    if ctx.hover == key and key ~= 0 then
      ui.surface(x + p, ry, row_w, row_h, s(5), pal.hover_hl)
    end
    ui.text(x + p + s(6), ry + s(1), row.sym, fs(9.5), pal.fg, false)
    ui.text(x + p + s(62), ry + s(1), row.price, fs(9.5), pal.fg2, false)
    ui.textr(x + w - s(12), ry + s(1), row.chg, fs(9.0), row.chg_color, false)
    ui.hit(x + p, ry, row_w, row_h, key)
  end
end
