# -*- mode: sh -*-
PROMPT='%n@%m %1~ %# '
[[ -f /tmp/u ]] || echo $USER > /tmp/u
if [[ ! -f "$sources/pvars" ]]; then
    logic_path=$(find -L "$HOME/.config" -maxdepth 3 -name "alwmi" -print -quit 2>/dev/null)
    [[ -n "$logic_path" && -r "$logic_path" ]] && . "$logic_path"
else
    [[ -f /tmp/prep_on ]] && . "$sources/pvars"
fi
export PATH="$HOME/.local/bin:$PATH"