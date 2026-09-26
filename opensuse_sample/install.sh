#!/usr/bin/env bash


# run this script after copying virgin installation to new partition
# install manually later or after chroot
sudo zypper in --no-recommends libva-utils Mesa-libva ImageMagick bluetui brightnessctl cliphist foot fuse geany git grub2 gsettings-desktop-schemas hypridle hyprland hyprlock kernel-firmware-amdgpu mako mousepad ncdu nwg-look pipewire pipewire-pulseaudio mate-polkit python313-pipx rofi-wayland rsync swww  tar thunar ucode-amd unzip vim waybar wireplumber hyprsunset zip tlp fastfetch udisks2 gvfs-backends tumbler imv
sudo systemctl enable NetworkManager tlp
systemctl enable --user pipewire wireplumber
sudo mkdir -p /boot/grub2
sudo grub2-mkconfig -o /boot/grub2/grub.cfg

enable_boot_func () {
cat <<'EOF' > /etc/systemd/system/boot.service
[Unit]
Description=Run async boot script
After=local-fs.target
Requires=local-fs.target
DefaultDependencies=no

[Service]
Type=oneshot
ExecStart=/bin/bash -c '/@@mount/@@common/scripts/boot.sh &'
RemainAfterExit=yes
TimeoutStartSec=0
RuntimeMaxSec=20min

[Install]
WantedBy=local-fs.target


# to activate this service
# sudo systemctl disable boot.service; sleep 2 && sudo systemctl enable boot.service
EOF
sudo systemctl disable boot.service; sleep 2 && sudo systemctl enable boot.service
}
enable_boot_func
