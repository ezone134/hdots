-- notes.lua — the Notes card's entire UI layer (LuaJIT host, QUICKSHELL_MODEL
-- §Lua). The engine (Rust) is backend + renderer; this module paints the card
-- once per frame through `ctx.ui.*`, the exact same Cmd components the Rust
-- drawers emit, so geometry is byte-identical. Geometry of record is
-- draw_notes_card / draw_notes_composer in
--   src/shell/panels/cards/notes.rs
-- The parity test `the_notes_lua_module_paints_exactly_like_the_rust_drawer`
-- (src/lua/mod.rs) asserts this module produces IDENTICAL commands for every
-- input stage; once that holds and this module is live, the Rust drawer (and
-- notes.ron) are DELETED.
--
-- Coordinates are DEVICE px: `ctx.s(n)` scales a base px by the ui scale,
-- `ctx.fs(n)` scales a base font size (rounded, matching the engine's fs()).

-- bullets / glyphs (icon font code points, UTF-8 encoded as byte escapes)
local BULLET = "\239\131\182" -- U+F0F6 list-square
local ADDES = "\239\129\167"  -- U+F067 plus

-- ── utf8 helpers (LuaJIT has no utf8 stdlib) — mirror Rust chars().count() ──
local function u8len(s)
  local n = 0
  for i = 1, #s do
    local b = string.byte(s, i)
    if b < 128 or b >= 192 then n = n + 1 end
  end
  return n
end
local function u8sub(s, k)
  if k <= 0 or #s == 0 then return "" end
  local i, c = 1, 0
  while i <= #s and c < k do
    local b, l = string.byte(s, i), 1
    if b >= 240 then l = 4 elseif b >= 224 then l = 3 elseif b >= 192 then l = 2 end
    c = c + 1; i = i + l
  end
  return string.sub(s, 1, i - 1)
end

-- chars first..last (1-based) — the char-window the Rust wrap_body needs
local function u8slice(s, first, last)
  if first > last or #s == 0 then return "" end
  local i, c, start_b, end_b = 1, 0, nil, nil
  while i <= #s do
    local b, l = string.byte(s, i), 1
    if b >= 240 then
      l = 4
    elseif b >= 224 then
      l = 3
    elseif b >= 192 then
      l = 2
    end
    c = c + 1
    if c == first then start_b = i end
    if c == last then
      end_b = i + l - 1
      break
    end
    i = i + l
  end
  if start_b == nil then return "" end
  return string.sub(s, start_b, end_b)
end

-- ── word-wrap — mirrors Shell::wrap_body (max_lines hard breaks) ──────────
local function wrap_body(text, max_chars, max_lines)
  local out = {}
  for para in string.gmatch(text, "[^\n]*") do
    local cur = ""
    for word in string.gmatch(para, "%S+") do
      local wc = u8len(word)
      if cur ~= "" and u8len(cur) + 1 + wc > max_chars then
        table.insert(out, cur)
        cur = ""
        if #out == max_lines then return out end
      end
      if wc > max_chars then
        local i = 1
        while i <= wc do
          table.insert(out, u8slice(word, i, math.min(wc, i + max_chars - 1)))
          if #out == max_lines then return out end
          i = i + max_chars
        end
        cur = ""
      else
        if cur ~= "" then cur = cur .. " " end
        cur = cur .. word
      end
    end
    if cur ~= "" then
      table.insert(out, cur)
      if #out == max_lines then return out end
    end
  end
  return out
end

-- ── the composer (title + body stages, input == 0 / input == 1) ────────────
local function draw_composer(ctx, api)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local py = ctx.y + s(30)
  local ph = math.max(ctx.h - s(30) - s(6), 20)
  ui.surface(ctx.x + s(8), py, ctx.w - s(16), ph, s(10), ui.mix(pal.hover, pal.fg, 0.06))

  -- title field — single line
  local ty = py + s(19)
  local t_active = (api.input == 0)
  ui.text(ctx.x + s(20), py + s(8), "Title", fs(8), pal.fg3, false)
  ui.surface(ctx.x + s(18), ty, ctx.w - s(36), s(18), s(6), t_active and pal.hover_hl or pal.hover)
  local t_shown = ""
  if api.title == "" and not t_active then
    t_shown = "Title…"
  else
    t_shown = u8sub(api.title, 40)
  end
  local tcw = u8len(t_shown) * fs(9) * 0.62
  ui.text(ctx.x + s(26), ty + s(4), t_shown, fs(9), (t_active or api.title == "") and pal.fg or pal.fg2, false)
  if t_active then
    ui.surface(ctx.x + s(25) + tcw, ty + s(5), s(1.4), s(11), s(0.7), pal.acc)
  end

  -- body field — a tall multi-line area
  local b_active = (api.input == 1)
  local body_label_y = ty + s(18) + s(10)
  local by = body_label_y + s(11)
  local bh = math.max(py + ph - s(24) - by, s(34))
  ui.text(ctx.x + s(20), body_label_y, "Body", fs(8), pal.fg3, false)
  ui.surface(ctx.x + s(18), by, ctx.w - s(36), bh, s(8), b_active and pal.hover_hl or pal.hover)

  local fsb = fs(9)
  local lh = s(12)
  local max_chars = math.max(12, math.floor((ctx.w - s(48)) / (fsb * 0.62)))
  local lines = wrap_body(api.body, max_chars, 6)
  -- windows like the scene's Rows: clip a line once ry+row_h leaves the card
  local fit = math.max(1, math.floor((ctx.h + 0.5 - s(95)) / lh))
  local n = math.min(#lines, fit)
  if n == 0 then
    if not b_active then
      ui.text(ctx.x + s(26), by + s(7), "Body…", fsb, pal.fg3, false)
    else
      ui.surface(ctx.x + s(25), by + s(8), s(1.4), s(11), s(0.7), pal.acc)
    end
  else
    for li = 1, n do
      local ly = by + s(7) + (li - 1) * lh
      ui.text(ctx.x + s(26), ly, lines[li], fsb, b_active and pal.fg or pal.fg2, false)
      if b_active and li == n then
        local cx = ctx.x + s(25) + u8len(lines[li]) * fsb * 0.62
        ui.surface(cx, ly + s(1), s(1.4), s(11), s(0.7), pal.acc)
      end
    end
  end

  -- save `+` — body stage only
  if b_active then
    local sbx = ctx.x + ctx.w - s(22)
    local sby = py + ph - s(18)
    local hov = (ctx.hover == api.keys.save)
    ui.surface(sbx - s(10), sby - s(10), s(20), s(20), s(10), hov and pal.hover_hl or pal.hover)
    ui.textc(sbx, sby - s(4.5), ADDES, fs(9.5), hov and pal.acc or pal.fg, true)
    ui.hit(sbx - s(11), sby - s(11), s(22), s(22), api.keys.save)
  end
  ui.text(ctx.x + s(20), py + ph - s(16), "↵ next field · ↵ save · esc cancel", fs(7.5), pal.fg3, false)
end

-- ── main draw — invoked once per frame by the host ─────────────────────────
function draw(ctx)
  local s, fs, ui, pal = ctx.s, ctx.fs, ctx.ui, ctx.pal
  local api = ctx.api

  -- shared card header (matches card_header, pad 14) unless the scene owns it
  if not api.scene_owns_header then
    ui.title(ctx.x + s(14), ctx.y + s(9), "Notes", fs(11.5), pal.fg)
    ui.captionr(ctx.x + ctx.w - s(14), ctx.y + s(11), tostring(api.count), fs(8.5), pal.fg3)
  end

  if api.input ~= nil then
    draw_composer(ctx, api)
    return
  end

  -- list stage
  if not api.scene_owns_rows then
    local row_h = s(24)
    local visible = math.max(1, math.floor((ctx.h - s(30) - s(34)) / row_h))
    local start = math.min(api.scroll, math.max(0, api.count - visible))
    for j = 0, visible - 1 do
      local it = api.lists[start + j + 1]
      if it == nil then break end
      local ry = ctx.y + s(32) + j * row_h
      ui.text(ctx.x + s(14), ry, BULLET, fs(9), pal.fg3, true)
      ui.text(ctx.x + s(28), ry, u8sub(it.title, 36), fs(9.5), pal.fg, false)
      if #it.body > 0 then
        ui.text(ctx.x + s(36), ry + s(12), u8sub(it.body, 44), fs(8), pal.fg2, false)
      end
      ui.surface(ctx.x + s(12), ry + row_h - s(1), ctx.w - s(24), s(1), s(0.5), pal.hover)
    end
  end
  if api.count == 0 then
    ui.textc(ctx.x + ctx.w / 2, ctx.y + ctx.h / 2 - s(10), "No notes", fs(9.5), pal.fg, false)
  end
  -- add `+` — bottom-right corner
  local pbx = ctx.x + ctx.w - s(20)
  local pby = ctx.y + ctx.h - s(18)
  local hov = (ctx.hover == api.keys.input)
  ui.surface(pbx - s(10), pby - s(10), s(20), s(20), s(10), hov and pal.hover_hl or pal.hover)
  ui.textc(pbx, pby - s(4.5), ADDES, fs(9.5), hov and pal.acc or pal.fg, true)
  ui.hit(pbx - s(11), pby - s(11), s(22), s(22), api.keys.input)
end