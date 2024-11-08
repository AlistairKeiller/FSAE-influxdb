#!/bin/bash
source install_influx.sh
sudo cp 80-can.network /etc/systemd/network/80-can.network
sudo cp data.service /etc/systemd/system/data.service

# for USB gadget
sudo touch /boot/firmware/ssh
# setup with /boot/firmware/config.txt dtoverlay=dwc2
# setup with /boot/firmware/cmdline.txt modules-load=dwc2,g_ether