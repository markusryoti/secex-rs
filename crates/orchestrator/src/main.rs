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
    let _ = std::fs::remove_file(&vsock_uds_path);

    vm.initialize(&vsock_uds_path);
    let _child = vm.launch();

    println!("Firecracker started");

    // The vsock device bridges connections from the guest (CID 3) to this Unix socket
    println!(
        "Connecting to guest via Unix socket at {}...",
        vsock_uds_path
    );

    while !std::path::Path::new(&vsock_uds_path).exists() {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    println!(
        "Unix socket {} is now available. Attempting to connect...",
        vsock_uds_path
    );

    tokio::time::sleep(Duration::from_secs(5)).await;

    let mut stream = loop {
        let mut s = match UnixStream::connect(&vsock_uds_path).await {
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

    // let mut stream = loop {
    //     match UnixStream::connect(&vsock_uds_path).await {
    //         Ok(s) => break s,
    //         Err(e) => {
    //             // Log the error to see if it's "Connection Refused" (normal while booting)
    //             // or something else.
    //             println!("Failed to connect to vsock UDS: {}", e);
    //             tokio::time::sleep(Duration::from_millis(100)).await;
    //         }
    //     }
    // };

    // println!("Connection established with Firecracker proxy!");

    // // 3. MANDATORY HANDSHAKE: Tell Firecracker which guest port to connect to
    // // Firecracker listens on the UDS but needs to know where to route the traffic
    // let handshake = "CONNECT 5001\n";
    // stream.write_all(handshake.as_bytes()).await.unwrap();

    // 4. Read the response from Firecracker (e.g., "OK 5001\n")
    let mut response = [0u8; 32];
    let n = stream
        .read(&mut response)
        .await
        .expect("Failed to read handshake response");
    let resp_str = String::from_utf8_lossy(&response[..n]);

    if !resp_str.starts_with("OK") {
        panic!("Firecracker vsock handshake failed: {}", resp_str);
    }

    println!("Vsock circuit established to guest port 5001!");

    // --- Communication Logic ---
    // Send your Hello message
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
