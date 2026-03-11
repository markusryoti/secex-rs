use std::{sync::Arc, time::Duration};

use axum::{Router, routing::get};
use tokio::{signal, sync::Mutex};
use tracing::{error, info};

mod firecracker;
mod network;
mod vm;
mod vm_handle;
mod vm_store;
mod vsock;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    network::setup_ip_forwarding().expect("Failed to setup forwarding");

    let store = Arc::new(Mutex::new(vm_store::VmStore::new()));

    let vm1 = vm::spawn_vm(store.lock().await.len() + 1);
    let id1 = vm1.id.clone();

    store.lock().await.add_vm(&id1, vm1);

    let vm2 = vm::spawn_vm(store.lock().await.len() + 1);
    let id2 = vm2.id.clone();

    store.lock().await.add_vm(&id2, vm2);

    let vm1 = store.lock().await.get_vm(&id1).unwrap();
    let vm2 = store.lock().await.get_vm(&id2).unwrap();

    let handles: Vec<_> = [vm1, vm2]
        .into_iter()
        .map(|vm| tokio::spawn(handle_vm(vm)))
        .collect();

    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    info!("Starting axum");

    tokio::select! {
        result = axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()) => {
            if let Err(e) = result {
                error!("Axum error: {}", e);
            }
            info!("Axum shut down, stopping VMs...");
        }

        result = futures::future::join_all(handles) => {
            result.into_iter().filter_map(|r| r.err())
            .for_each(|e| error!("Error from task: {}", e));
        }
    }

    info!("Stopping orchestrator, removing VM's");

    store.lock().await.remove_vm(&id1);
    store.lock().await.remove_vm(&id2);

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

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    // On non-Unix platforms, SIGTERM isn't available — use a future that never resolves.
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { info!("Received Ctrl+C, shutting down...") },
        _ = terminate => { info!("Received SIGTERM, shutting down...") },
    }
}
