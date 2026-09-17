# if something broke first try running link.sh no need to install everything from scratch
SCRIPT_DIR="${BASH_SOURCE[0]%/*}"
# run link script
$SCRIPT_DIR/link.sh
$SCRIPT_DIR/link_udev_rule.sh
$SCRIPT_DIR/setup_systemd_service.sh

#
# add more logic to install required fonts and themes etc

sudo zypper ref
sudo zypper in hyprlock hypridle ImageMagick rofi-wayland wfl-recorder udisks2 rsync thunar tumbler sypper mako mate-polkit libnotify-tools hyprsunset cliphist bluetui brightnessctl \
cliphist gvfs-backends
