use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tracing::info;

mod network;
mod vm;

async fn wait_for_socket(vsock_uds_path: &str) {
    while !std::path::Path::new(&vsock_uds_path).exists() {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    info!(
        "Unix socket {} is now available. Attempting to connect...",
        vsock_uds_path
    );
}

async fn connect_to_vsock(vsock_uds_path: &str) -> UnixStream {
    info!(
        "Connecting to guest via Unix socket at {}...",
        vsock_uds_path
    );

    let stream = loop {
        let mut s = match UnixStream::connect(vsock_uds_path).await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to connect to vsock UDS: {}", e);
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };

        // Send the handshake immediately
        if let Err(_) = s.write_all(b"CONNECT 5001\n").await {
            continue;
        }

        // Read response - if we get "OK", we are truly connected to the guest
        let mut buf = [0u8; 32];
        match s.read(&mut buf).await {
            Ok(n) if n > 0 => {
                let resp = String::from_utf8_lossy(&buf[..n]);
                if resp.contains("OK") {
                    info!("Guest is ready and handshake successful!");
                    break s;
                }
            }
            _ => {}
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };

    info!("Vsock circuit established to guest port 5001!");

    stream
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

async fn handle_incoming<T: AsyncReadExt + Unpin>(mut stream: T) {
    loop {
        let message = protocol::recv_msg(&mut stream)
            .await
            .expect("Failed to receive message");

        match message {
            protocol::Message::Hello => {
                info!("Guest said Hello!");
            }
            protocol::Message::CommandOutput(output) => {
                info!("Received command output from guest: {}", output.output);
                break;
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
}

async fn initial_commands<T: AsyncWriteExt + Unpin>(stream: &mut T) {
    send_hello(stream).await;
    send_command(stream).await;
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let store = &mut vm::VmStore::new();

    info!("Created VM store");

    let vm = vm::VmConfig::new(store.len() + 1);
    let vm_id = vm.id.clone();

    info!(
        "Created VM configuration for {}. Setting up network interface...",
        vm.id
    );

    store.add_vm(vm);

    let vm = store
        .get(&vm_id)
        .expect("Failed to retrieve VM configuration");

    let vsock_uds_path = format!("/tmp/vsock-{}.sock", vm_id);
    match std::fs::remove_file(&vsock_uds_path) {
        Ok(_) => info!("Removed existing vsock UDS at {}", vsock_uds_path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            info!("No existing vsock UDS at {}, proceeding...", vsock_uds_path)
        }
        Err(e) => panic!("Failed to remove existing vsock UDS: {}", e),
    }

    vm.initialize(&vsock_uds_path);
    let _child = vm.launch();

    info!(
        "VM {} launched with API socket at {:?} and TAP device {}",
        vm.id, vm.api_socket, vm.tap
    );

    wait_for_socket(&vsock_uds_path).await;

    let stream = connect_to_vsock(&vsock_uds_path).await;

    let (reader, mut writer) = stream.into_split();

    let handle = tokio::spawn(async {
        handle_incoming(reader).await;
    });

    initial_commands(&mut writer).await;

    handle.await.unwrap();

    info!("Initiating shutdown sequence...");
    send_shutdown(&mut writer).await;
}
