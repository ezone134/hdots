#!/usr/bin/env bash

hypr_global=/tmp/hypr_global

link_it() {
    local src="$1" dest="$2"
    local rnd="$RANDOM"

    # Use built-in -ef operator to check if paths resolve to the same file/symlink (replaces readlink)
    [[ "$src" -ef "$dest" ]] || {
        [[ -L "$dest" ]] && sudo mv "$dest" "/tmp/moved_deleted.$rnd" 2>/dev/null
        sudo mv "$dest" "$dest.bak.$rnd" 2>/dev/null
        sudo ln -s "$src" "$dest" 2>/dev/null
    }
}

link_it $hypr_global/99-power-source.rules /etc/udev/rules.d/99-power-source.rules
