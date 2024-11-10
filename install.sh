#!/bin/bash
source install_influx.sh
sudo cp 80-can.network /etc/systemd/network/80-can.network
sudo cp data.service /etc/systemd/system/data.service
sudo systemctl enable systemd-networkd
