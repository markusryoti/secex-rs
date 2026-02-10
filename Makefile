orchestrator:
	cargo build --bin orchestrator
	sudo target/debug/orchestrator

build-init:
	cargo build --release --package init --target x86_64-unknown-linux-musl
	cp target/x86_64-unknown-linux-musl/release/init rootfs/init

build-rootfs:
	tools/build-rootfs-image.sh

kill-fc-processes:
	pkill -f firecracker || true

remove-fc-sockets:
	sudo find /tmp -name "*firecracker*.socket" -exec bash -c 'echo "Removing {}"; sudo rm -f {}' \;

remove-vsock-sockets:
	sudo find /tmp -name "vsock-vm-*.sock" -exec bash -c 'echo "Removing {}"; sudo rm -f {}' \;