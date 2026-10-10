#!/usr/bin/env bash

SCRIPT_DIR="${BASH_SOURCE[0]%/*}"
export dots="$HOME/.config/hdots"

[[ "$SCRIPT_DIR" == "$HOME/.config" || "$SCRIPT_DIR" == "$HOME/.config/"* ]] && {
    echo "Do not run from inside $HOME/.config.
    Put hdots in $HOME/.local/share/ and run from there."
    exit 1
}

symlinks=(
    "$dots:$SCRIPT_DIR"
    "$HOME/.config/hypr:$dots/.config/hypr"
)

for pair in "${symlinks[@]}"; do
    l="${pair%%:*}"; r="${pair##*:}"
    mkdir -p "$r" "${l%/*}"
    [[ "$l" -ef "$r" ]] || { mv "$l" "$l.$RANDOM.delme" 2>/dev/null; ln -s "$r" "$l"; }
done

update_config() {
    local file="$1" line="$2" found=0
    [[ -f "$file" ]] && while IFS= read -r l || [[ -n "$l" ]]; do
        [[ "$l" == "$line" ]] && { found=1; break; }
    done < "$file"
    (( found )) || {
        [[ -f "$file" ]] && cp "$file" "$file.$RANDOM.bak"
        printf '%s\n%s\n' "$line" "$(< "$file")" > "$file"
    }
}

# Fixed: Added semicolon before 'then' and formatted cleanly
update_config ~/.bashrc 'if [[ ! -f "$sources/misc/env_bash" ]]; then
    d_id=hdots
    . $HOME/.config/${d_id}/sources/misc/env_bash
    else
    . $sources/misc/env_bash
fi'

update_config ~/.profile '. ~/.bashrc'

mkdir -p "$HOME/.config/hypr"
cat << 'EOF' > "$HOME/.config/hypr/hyprland.lua"
require('env')
require('keybinds')
hl.on("hyprland.start", function()
    hl.exec_cmd("\$dots/sources/bin/launch")
end)
require('primary')
require('luas/monitors')
require('luas/general')
require('luas/border')
require('luas/blur')
require('luas/shadow')
require('luas/opacity')
require('luas/shader')
require('overrides')
EOF

echo "All linking logic finished"
