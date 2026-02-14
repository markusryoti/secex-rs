use protocol::WorkspaceRunOptions;
use std::{sync::Arc, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{error, info};

mod firecracker;
mod network;
mod vm;
mod vsock;

async fn handle_vm_lifecycle(vm: Arc<vm::VmConfig>) {
    let vsock_uds_path = format!("/tmp/vsock-{}.sock", vm.id);

    vsock::remove_existing_vsock(&vsock_uds_path);

    vm.initialize(&vsock_uds_path);
    let _child = vm.launch();

    info!(
        "VM {} launched with API socket at {:?} and TAP device {}",
        vm.id, vm.api_socket, vm.tap
    );

    vsock::wait_for_socket(&vsock_uds_path).await;

    let stream = vsock::connect_to_vsock(&vsock_uds_path).await;
    let (reader, mut writer) = stream.into_split();

    let handle = tokio::spawn(async {
        handle_incoming(reader).await;
    });

    initial_commands(&mut writer).await;

    handle.await.unwrap();
}

async fn handle_incoming<T: AsyncReadExt + Unpin>(mut stream: T) {
    loop {
        let message = match protocol::recv_msg(&mut stream).await {
            Ok(m) => m,
            Err(e) => {
                error!("Error receiving message: {}", e);
                return;
            }
        };

        match message {
            protocol::Message::Hello => {
                info!("Guest said Hello!");
            }
            protocol::Message::CommandOutput(output) => {
                info!("Received command output from guest: {}", output.output);
            }
            m => info!("Received other message: {:?}", m),
        }
    }
}

async fn send_shutdown<T: AsyncWriteExt + Unpin>(stream: &mut T) {
    protocol::send_msg(stream, protocol::Message::Shutdown)
        .await
        .unwrap();

    info!("Sent Shutdown message to guest, closing connection...");

    let _ = stream.shutdown();
}

async fn initial_commands<T: AsyncWriteExt + Unpin>(stream: &mut T) {
    send_hello(stream).await;
    send_command(stream).await;
    send_run_workspace(stream).await;

    tokio::time::sleep(Duration::from_secs(5)).await;

    send_shutdown(stream).await;
}

async fn send_hello<T: AsyncWriteExt + Unpin>(stream: &mut T) {
    protocol::send_msg(stream, protocol::Message::Hello)
        .await
        .unwrap();

    info!("Sent Hello message to guest");
}

async fn send_command<T: AsyncWriteExt + Unpin>(stream: &mut T) {
    let cmd = protocol::RunCommand {
        command: "echo".to_string(),
        args: vec!["Hello from orchestrator!".to_string()],
        env: std::collections::HashMap::new(),
        working_dir: None,
    };

    protocol::send_msg(stream, protocol::Message::RunCommand(cmd))
        .await
        .unwrap();

    info!("Sent RunCommand message to guest");
}

async fn send_run_workspace<T: AsyncWriteExt + Unpin>(stream: &mut T) {
    protocol::tar::tar_workspace("workspace", "workspace.tar").expect("Failed to create tarball");

    let data = std::fs::read("workspace.tar").expect("Failed to read tarball");

    protocol::send_msg(
        stream,
        protocol::Message::RunWorkspace(WorkspaceRunOptions {
            data,
            entrypoint: "run.sh".to_string(),
        }),
    )
    .await
    .expect("Failed to run in workspace");

    info!("Sent RunWorkspace message to guest");
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let mut store = vm::VmStore::new();
    info!("Created VM store");

    let vm = Arc::new(vm::VmConfig::new(store.len() + 1));

    info!(
        "Created VM configuration for {}. Setting up network interface...",
        vm.id
    );

    let id = vm.id.clone();

    store.add_vm(vm.clone());

    handle_vm_lifecycle(vm).await;

    store.remove_vm(&id);
}
