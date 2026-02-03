orchestrator:
	cargo run --bin orchestrator

build-init:
	cargo build --release --package init --target x86_64-unknown-linux-musl
	cp target/x86_64-unknown-linux-musl/release/init rootfs/init

build-rootfs:
	tools/build-rootfs-image.sh