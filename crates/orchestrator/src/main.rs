use std::{sync::Arc, time::Duration};

use tokio::sync::Mutex;

use crate::vm::VmConfig;

mod firecracker;
mod network;
mod vm;
mod vm_store;
mod vsock;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    network::setup_ip_forwarding().expect("Failed to setup forwarding");

    let store = Arc::new(Mutex::new(vm_store::VmStore::new()));

    let vm = vm::VmConfig::new(store.lock().await.len() + 1);
    let id = vm.id.clone();
    store.lock().await.add_vm(&id, vm);

    let vm = store.lock().await.get_vm(&id).unwrap();

    handle_vm(vm).await;

    store.lock().await.remove_vm(&id);

    network::cleanup_ip_forwarding().expect("Failed to cleanup forwarding");
}

async fn handle_vm(vm: Arc<Mutex<VmConfig>>) {
    {
        let vm = vm.lock().await;
        vm.initialize();
        vm.launch().await;
        vm.connect().await;
    }

    vm.lock()
        .await
        .send_message(protocol::Message::Hello)
        .await
        .expect("Failed to send hello command");

    let curl_cmd = protocol::RunCommand {
        command: "curl".to_string(),
        args: vec!["-v".to_string(), "http://example.com".to_string()],
        env: std::collections::HashMap::new(),
        working_dir: None,
    };

    vm.lock()
        .await
        .send_message(protocol::Message::RunCommand(curl_cmd))
        .await
        .expect("Failed to send curl command");

    protocol::tar::tar_workspace("workspace", "workspace.tar").expect("Failed to create tarball");

    let data = std::fs::read("workspace.tar").expect("Failed to read tarball");

    let ws_msg = protocol::Message::RunWorkspace(protocol::WorkspaceRunOptions {
        data,
        entrypoint: "run.sh".to_string(),
    });

    vm.lock()
        .await
        .send_message(ws_msg)
        .await
        .expect("Failed to send workspace command");

    tokio::time::sleep(Duration::from_secs(5)).await;

    vm.lock()
        .await
        .send_message(protocol::Message::Shutdown)
        .await
        .expect("Shutdown message sending failed");

    vm.lock().await.cleanup();
}
