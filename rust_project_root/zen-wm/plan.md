
i wanna build a max speed max efficiency window manager hyprland clone but in rust taht also has niri like worspace overview support, saturation control from day 1

fuzzel as default launcher
alacritty as default terminal

mod+tab opens workspace overview (during overview it grabs all key inputs)
when on over view mode:
super+up,down, left, right .... navigates thru workspace not holding super also works

when not in over view mode:
super+right opens right sided apps and super+left opens left sided apps
super + up opens prev workspace super+down opens nex workspace (only if not empty workspace)

minimum possible overhead least over head...


make it very modular... or very easy to add remove feature its gonna be huge project in future so ... it must be easy to understand and will be coded entirely by AI agents


it can reload config live... just like hyprland

include all beautiful eye candy effects like hyprland has ..
blur
shadow
animation
opacity
window rounded corner

===== CLARIFIED VISION (2026-08-16) =====
Think of it as: a Hyprland rewrite in Rust, with niri's scroll-over-view ported in.
- SCROLLING layout is the DEFAULT (full-width vertical window column per workspace)
- workspaces scroll vertically; overview = niri-style vertical strip of workspace
  thumbnails (grid still available via config)
- layouts: scrolling (default) / master / (future) dwindle
- live config reload: DONE (mtime poll, hot-applies everything)

think of it as 
it a tiling window manager hyprland rewrite in rust and porting niri's scroll over view to it
it has scrolling layout as default