#!/bin/sh

set -e

docker build -t vm-rootfs rootfs

container_id=$(docker create vm-rootfs)
docker export "$container_id" | tar -C rootfs -xvf -
docker rm "$container_id"

# Create ext4 image
dd if=/dev/zero of=rootfs.ext4 bs=1M count=128
mkfs.ext4 rootfs.ext4
sudo mount rootfs.ext4 /mnt
sudo cp -r rootfs/* /mnt
sudo umount /mnt
