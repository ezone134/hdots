# BUGS

Fixed (2026-09-14, later):

- **Hover expanded to an EMPTY dashboard (no cards, background only)** — the
  morph's size handshake deadlocked on a fractional tween tail. The final
  tween commit (e.g. 1009.79×494.79) was compared against the last acked
  configure (1009×494) with a 0.5 px f32 epsilon; since layer-shell w/h are
  truncated u32, that IS the same surface size, but 0.79 > 0.5 slipped past
  the "already configured" guard in `commit_size` and re-sent
  `set_size(1009,494)` — a no-op commit Hyprland never acks. `size_pending`
  stuck → `Shell::tick` refuses to finish the anim while pending →
  `maybe_render` drops every frame while the anim is up. Worse, the 800 ms
  timeout cleared pending, the next tick cleared the anim and RE-committed
  the same never-acked size (stuck again), and `morph_tick`'s stop condition
  then removed the morph timer while pending was set — nothing left could
  ever unstick it. Fraction-dependent (triggers when the tween tail lands in
  the (0.5,1.0) px band — true for this monitor × config), so it fired on
  every hover. Fix: `commit_size` now compares truncated sizes
  (`size_is_configured`), and the morph timer's stop condition also requires
  `!size_pending` so the timer always outlives a stuck commit. Regression
  tests `final_fractional_tween_size_counts_as_configured`,
  `different_size_is_not_configured`, `tick_holds_anim_while_size_pending`
  (tests 107 → 110). Verified live: expand now paints the dashboard
  (133 frames at 1009×494, scene 8 → 97 cmds).

Fixed (2026-09-14):

- **Edit-mode ✕ not clickable on a scrolled board** — clicking a card's
  top-right ✕ started a drag instead of parking the card whenever the board
  was scrolled: the hit rects in `edit_press` were computed in three
  different coordinate spaces (grip scroll-corrected, body y-scroll-only,
  ✕ uncorrected) while the draw loop scrolls all content by both axes. All
  three now resolve via `scrolled()`; `card_close_px` is scale-aware
  (grid_scale ≠ 1 was silently off too); the resize drag compensates scroll
  like the move drag already did. Regression test
  `close_button_hit_rect_follows_board_scroll` (107 tests green).

Fixed (2026-09-05):

- **Launch pill not immediately expandable** — "starts in an un-expandable
  pill state". The hover rule only ran on pointer events / the 60s tick, so a
  cursor already resting over the bar (or a pointer-enter consumed during the
  initial roundtrip before the first configure) left the pill collapsed until
  the mouse moved. Fix: `App::start` now runs `reconcile_hover()` at boot and
  again ~300 ms later once the layer maps, so a resting cursor expands the
  dashboard immediately.

- **Pill frozen in long-running sessions** — a size commit that was never
  acked left `size_pending` set, and `maybe_render` silently drops every redraw
  while pending; the only unstick was the 60 s clock tick, so the pill could
  stall up to a minute. Fix: `morph_tick` now calls `check_size_timeout()`
  every tick, so a stuck commit un-sticks within the 800 ms budget.

- **`colors reload` sometimes breaks the shell (needs restart)** — a channel
  switch adopted a new master config but never re-applied the layer geometry
  (anchor/floating/reserve) and could start a morph without a timer driving
  it, leaving the bar at the old size/position until a hover happened to fix
  it. Fix: `sync_per_state_config` now adopts the bar geometry fields and
  always re-packs/re-fits; the IPC handler re-applies `apply_bar_geometry()`
  and arms the morph timer when anything changed.

Re-open / unfixed:

- (none known)