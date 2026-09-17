PROMPT='%n@%m %1~ %# '
[[ -e $hypr_base/hypr_misc/misc/shell_body ]] && . $hypr_base/hypr_misc/misc/shell_body
[[ -e /tmp/hypr_auto_${USER} ]] || {
    . ~/.config/hdots/sources/helpers/create_dummy_configs
    :> /tmp/hypr_auto_${USER}
    hypr
}

# Antigravity CLI & Alacritty Terminal Configuration
export PATH="$HOME/.local/bin:$PATH"
if [ "$TERM" = "alacritty" ]; then
    export TERM=xterm-256color
fi
export COLORTERM=truecolor



# Added by Antigravity CLI installer
export PATH="/home/tw/.local/bin:$PATH"
