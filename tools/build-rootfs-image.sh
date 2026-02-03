#!/bin/sh

set -e

rm -rf build

docker build -t vm-rootfs rootfs

container_id=$(docker create vm-rootfs)
mkdir -p build/rootfs
docker export "$container_id" | tar -C build/rootfs -xvf -
docker rm "$container_id"

# Create ext4 image
dd if=/dev/zero of=build/rootfs.ext4 bs=1M count=128
mkfs.ext4 build/rootfs.ext4
sudo mount build/rootfs.ext4 /mnt
sudo cp -r build/rootfs/* /mnt
sudo umount /mnt
