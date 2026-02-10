use protocol;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

mod network;
mod vm;

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
    vm.initialize(&vsock_uds_path);
    let _child = vm.launch();

    println!("Firecracker started");

    // The vsock device bridges connections from the guest (CID 3) to this Unix socket
    println!(
        "Connecting to guest via Unix socket at {}...",
        vsock_uds_path
    );

    // Wait for the vsock socket to be created by Firecracker
    let mut attempts = 0;
    const MAX_ATTEMPTS: u32 = 50;
    let mut stream = loop {
        match UnixStream::connect(&vsock_uds_path).await {
            Ok(s) => break s,
            Err(_) => {
                if attempts >= MAX_ATTEMPTS {
                    panic!(
                        "Failed to connect to vsock Unix socket after {} attempts",
                        MAX_ATTEMPTS
                    );
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
                attempts += 1;
            }
        }
    };

    println!("Connected to vsock Unix socket");

    // Now wait for the guest to initiate a connection
    // The guest will connect to CID 2 port 5001, which bridges to this socket
    println!("Waiting for guest to connect...");

    // Read guest's Hello message
    let mut buffer = vec![0u8; 4];
    match tokio::time::timeout(Duration::from_secs(30), stream.read_exact(&mut buffer)).await {
        Ok(Ok(_)) => {
            let len = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
            let mut msg_buffer = vec![0u8; len];
            stream
                .read_exact(&mut msg_buffer)
                .await
                .expect("Failed to read message");
            println!("Received message from guest");
        }
        Ok(Err(e)) => eprintln!("Error reading guest message: {}", e),
        Err(_) => eprintln!("Timeout waiting for guest message"),
    }

    // Send Hello message to guest
    let env = protocol::Envelope {
        version: 1,
        message: protocol::Message::Hello,
    };

    let data = serde_json::to_vec(&env).expect("Error writing message");
    let len = (data.len() as u32).to_be_bytes();

    stream
        .write_all(&len)
        .await
        .expect("Failed to write length");
    stream.write_all(&data).await.expect("Failed to write data");

    println!("Sent Hello to guest");
    println!("Communication with guest established");
}
