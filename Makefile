orchestrator:
	cargo build --bin orchestrator
	target/debug/orchestrator

build-init:
	cargo build --release --package init --target x86_64-unknown-linux-musl
	cp target/x86_64-unknown-linux-musl/release/init rootfs/init

build-rootfs:
	tools/build-rootfs-image.sh

kill-fc-processes:
	pkill -f firecracker || true

clean-sockets:
	sudo rm -f /tmp/vsock-vm-*.sock /tmp/firecracker-*.socket; echo "Sockets cleaned"

list-sockets:
	ls -l /tmp | grep -e 'vsock-vm-.*\.sock' -e 'firecracker-.*\.socket' || echo "No sockets found"

remove-logs:
	find . -name "vm-*-firecracker.log" -type f -delete; echo "Log files removed"