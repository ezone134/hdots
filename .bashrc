# -*- mode: sh -*-
PROMPT='%n@%m %1~ %# '

export PATH="\
$HOME/.local/bin:\
$HOME/.opencode/bin:\
$HOME/.ante/bin:\
$HOME/.kilo/bin:\
$HOME/.cargo/bin:\
/home/appimages/node/bin:\
/home/appimages/Antigravity-x64/antigravity:\
/acc/common/hdots/rust_project_root/zen-shell/scripts:\
/acc/data/persist/user/.cargo/target/debug:\
$PATH"


if [[ -e "$hypr_base/hypr_misc/misc/shell" ]]; then
. $hypr_base/hypr_misc/misc/shell
else
. $HOME/.config/hdots/misc/shell
fi
