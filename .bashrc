# -*- mode: sh -*-
PROMPT='%n@%m %1~ %# '
if [[ ! -f "$sources/misc/env_bash" ]]; then
    path=("$HOME"/.config/*/states/s)
    d_id="${path[0]#$HOME/.config/}"
    d_id="${d_id%/states/s}"
    . $HOME/.config/${d_id}/sources/misc/env_bash
    else
    . $sources/misc/env_bash
fi

