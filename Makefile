orchestrator:
	cargo run --bin orchestrator

build-init:
	cargo build --package init --release
	cp target/release/init rootfs/init

build-rootfs:
	tools/build-rootfs-image.sh