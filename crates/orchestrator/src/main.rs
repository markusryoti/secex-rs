use std::{sync::Arc, time::Duration};

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

    vm.send_message(protocol::Message::Hello)
        .await
        .expect("Failed to send hello command");

    let curl_cmd = protocol::RunCommand {
        command: "curl".to_string(),
        args: vec!["-v".to_string(), "http://example.com".to_string()],
        env: std::collections::HashMap::new(),
        working_dir: None,
    };

    vm.send_message(protocol::Message::RunCommand(curl_cmd))
        .await
        .expect("Failed to send curl command");

    protocol::tar::tar_workspace("workspace", "workspace.tar").expect("Failed to create tarball");

    let data = std::fs::read("workspace.tar").expect("Failed to read tarball");

    let ws_msg = protocol::Message::RunWorkspace(protocol::WorkspaceRunOptions {
        data,
        entrypoint: "run.sh".to_string(),
    });

    vm.send_message(ws_msg)
        .await
        .expect("Failed to send workspace command");

    tokio::time::sleep(Duration::from_secs(5)).await;

    vm.send_message(protocol::Message::Shutdown)
        .await
        .expect("Shutdown message sending failed");

    vm.cleanup();

    store.remove_vm(&id);
}
