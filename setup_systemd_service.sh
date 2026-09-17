#!/usr/bin/env bash

echo "[Unit]
Description=Root Loop - Persistent Wait
# We remove ConditionPathExists so systemd actually tries to run it

[Service]
Type=simple
User=root
ExecStart=/tmp/hypr_global/loop

# --- THE PERSISTENCE MAGIC ---
# Restart regardless of why it failed (even if the file is missing)
Restart=always
# How often to check for the script (every 5 seconds)
RestartSec=30s

# Disable the "Start Limit" so it never gives up
StartLimitIntervalSec=0

[Install]
WantedBy=multi-user.target" | sudo tee /etc/systemd/system/loop.service

sudo systemctl disable loop.service
sudo systemctl enable loop.service
sudo systemctl start loop.service


sudo mkdir -p /etc/systemd/logind.conf.d
echo "[Login]
# Stop systemd from doing anything regardless of power state
HandleLidSwitch=ignore
HandleLidSwitchExternalPower=ignore
HandleLidSwitchDocked=ignore" | sudo tee /etc/systemd/logind.conf.d/lid.conf
