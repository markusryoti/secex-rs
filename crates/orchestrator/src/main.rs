use std::sync::Arc;

mod firecracker;
mod network;
mod vm;
mod vsock;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let tap_name = "tap0";
    let tap_ip = "172.16.0.1";
    let mask = "/30";

    network::setup_tap_device(tap_name, tap_ip, mask).expect("Tap setup failed");

    let mut store = vm::VmStore::new();

    let vm = Arc::new(vm::VmConfig::new(store.len() + 1));

    let id = vm.id.clone();

    store.add_vm(vm.clone());

    vm.initialize();
    vm.launch().await;
    vm.connect().await;

    store.remove_vm(&id);

    network::cleanup_tap_device(tap_name).expect("Failed to delete tap");
}
