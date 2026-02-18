use std::{sync::Arc, time::Duration};

use tokio::sync::Mutex;

mod firecracker;
mod network;
mod vm;
mod vm_handle;
mod vm_store;
mod vsock;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    network::setup_ip_forwarding().expect("Failed to setup forwarding");

    let store = Arc::new(Mutex::new(vm_store::VmStore::new()));

    let vm = vm::spawn_vm(store.lock().await.len() + 1);
    let id = vm.id.clone();

    store.lock().await.add_vm(&id, vm);

    let vm = store.lock().await.get_vm(&id).unwrap();

    let handle = tokio::spawn(handle_vm(vm));

    handle.await.expect("Failed to wait task handle");

    store.lock().await.remove_vm(&id);

    network::cleanup_ip_forwarding().expect("Failed to cleanup forwarding");
}

async fn handle_vm(vm: Arc<vm_handle::VmHandle>) {
    vm.start_vm().await.unwrap();

    let curl_cmd = protocol::RunCommand {
        command: "curl".to_string(),
        args: vec!["-v".to_string(), "http://example.com".to_string()],
        env: std::collections::HashMap::new(),
        working_dir: None,
    };

    vm.send_command(curl_cmd).await.unwrap();

    protocol::tar::tar_workspace("workspace", "workspace.tar").expect("Failed to create tarball");

    let data = std::fs::read("workspace.tar").expect("Failed to read tarball");

    let ws_cmd = protocol::WorkspaceRunOptions {
        data,
        entrypoint: "run.sh".to_string(),
    };

    vm.send_workspace_command(ws_cmd).await.unwrap();

    tokio::time::sleep(Duration::from_secs(5)).await;

    vm.shutdown().await.unwrap();
}
