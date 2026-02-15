use std::sync::Arc;

use crate::vm::VmStore;

mod firecracker;
mod network;
mod vm;
mod vsock;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    network::setup_ip_forwarding().expect("Failed to setup forwarding");

    let mut store = vm::VmStore::new();

    let vm = vm::VmConfig::new(store.len() + 1);
    let id = vm.id.clone();
    store.add_vm(Arc::new(vm));

    handle_vm(&id, &mut store).await;

    network::cleanup_ip_forwarding().expect("Failed to cleanup forwarding");
}

async fn handle_vm(id: &str, store: &mut VmStore) {
    let vm = store.get_vm(&id);
    let vm = vm.unwrap();

    vm.initialize();
    vm.launch().await;
    vm.connect().await;
    vm.cleanup();

    store.remove_vm(&id);
}
