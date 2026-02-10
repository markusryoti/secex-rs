use protocol;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

mod network;
mod vm;

async fn wait_for_socket(vsock_uds_path: &str) {
    while !std::path::Path::new(&vsock_uds_path).exists() {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    println!(
        "Unix socket {} is now available. Attempting to connect...",
        vsock_uds_path
    );
}

async fn connect_to_vsock(vsock_uds_path: &str) -> UnixStream {
    println!(
        "Connecting to guest via Unix socket at {}...",
        vsock_uds_path
    );

    let stream = loop {
        let mut s = match UnixStream::connect(vsock_uds_path).await {
            Ok(s) => s,
            Err(e) => {
                println!("Failed to connect to vsock UDS: {}", e);
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
                    println!("Guest is ready and handshake successful!");
                    break s;
                }
            }
            _ => {}
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };

    println!("Vsock circuit established to guest port 5001!");

    stream
}

async fn send_hello(stream: &mut UnixStream) {
    let env = protocol::Envelope {
        version: 1,
        message: protocol::Message::Hello,
    };

    let data = serde_json::to_vec(&env).unwrap();

    stream
        .write_all(&(data.len() as u32).to_be_bytes())
        .await
        .unwrap();
    stream.write_all(&data).await.unwrap();

    println!("Sent Hello message to guest");
}

async fn handle_incoming(stream: &mut UnixStream) {
    loop {
        let mut len_buf = [0u8; 4];
        if let Err(_) = stream.read_exact(&mut len_buf).await {
            println!("Connection closed by host.");
            break;
        }
        let len = u32::from_be_bytes(len_buf) as usize;

        let mut msg_buf = vec![0u8; len];
        stream
            .read_exact(&mut msg_buf)
            .await
            .expect("Failed to read message body");

        let envelope: protocol::Envelope =
            serde_json::from_slice(&msg_buf).expect("Failed to parse JSON");

        match envelope.message {
            protocol::Message::Hello => {
                println!("Guest said Hello!");
                break;
            }
            _ => println!("Received other message"),
        }
    }
}

async fn send_shutdown(stream: &mut UnixStream) {
    let shutdown_msg = protocol::Envelope {
        version: 1,
        message: protocol::Message::Shutdown,
    };

    let shutdown_data = serde_json::to_vec(&shutdown_msg).unwrap();
    stream
        .write_all(&(shutdown_data.len() as u32).to_be_bytes())
        .await
        .unwrap();
    stream.write_all(&shutdown_data).await.unwrap();

    println!("Sent Shutdown message to guest, closing connection...");
}

#[tokio::main]
async fn main() {
    let store = &mut vm::VmStore::new();

    let vm = vm::VmConfig::new(store.len() + 1);
    let vm_id = vm.id.clone();

    store.add_vm(vm);

    let vm = store
        .get(&vm_id)
        .expect("Failed to retrieve VM configuration");

    let vsock_uds_path = format!("/tmp/vsock-{}.sock", vm_id);
    let _ = std::fs::remove_file(&vsock_uds_path);

    vm.initialize(&vsock_uds_path);
    let _child = vm.launch();

    println!("Firecracker started");

    wait_for_socket(&vsock_uds_path).await;

    let mut stream = connect_to_vsock(&vsock_uds_path).await;

    send_hello(&mut stream).await;
    handle_incoming(&mut stream).await;

    println!("Initiating shutdown sequence...");
    send_shutdown(&mut stream).await;
}
