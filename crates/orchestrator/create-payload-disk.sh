#!/bin/bash

set -e

# # 1. Create a directory
# payload/
# ├── program
# ├── input.json
# └── config.json

# 2. Create filesystem image
dd if=/dev/zero of=payload.ext4 bs=1M count=64
mkfs.ext4 payload.ext4
sudo mount payload.ext4 /mnt
sudo cp -r payload/* /mnt
sudo umount /mnt