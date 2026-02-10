use protocol;
use std::{fs, time::Duration};
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::UnixListener,
};

use tokio::io::AsyncBufReadExt;

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

    vm.initialize();
    let child = vm.launch();

    println!("Firecracker started with PID: {}", child.id());

    let uds_path = format!("/tmp/vsock-{}.sock", vm_id);

    // Wait for Firecracker to create the vsock socket
    let mut attempts = 0;
    while !std::path::Path::new(&uds_path).exists() && attempts < 50 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        attempts += 1;
    }

    match fs::remove_file(&uds_path) {
        Ok(_) => println!("Removed existing socket at {}", uds_path),
        Err(err) => println!("No existing socket to remove: {}", err),
    };

    let listener = UnixListener::bind(&uds_path).expect("Bind to vsock");
    // let (mut stream, _) = listener
    //     .accept()
    //     .await
    //     .expect("Failed to accept vsock connection");

    loop {
        let (mut stream, _) = listener.accept().await.expect("Failed to accept conn");

        let h = tokio::spawn(async move {
            let (reader, mut writer) = stream.split();
            let mut buf_reader = BufReader::new(reader);
            let mut line = String::new();

            // 1. Firecracker sends "CONNECT <PORT>\n" first
            buf_reader.read_line(&mut line).await.unwrap();
            println!("Guest connected via port: {}", line.trim());

            // 2. Now you can use buf_reader/writer to talk to the guest
            // ... your logic here ...

            return;
        });

        h.await.unwrap();
        break;
    }

    // let env = protocol::Envelope {
    //     version: 1,
    //     message: protocol::Message::Hello,
    // };

    // let data = serde_json::to_vec(&env).expect("Error writing message");
    // let len = (data.len() as u32).to_be_bytes();

    // stream
    //     .write_all(&len)
    //     .await
    //     .expect("Failed to write length");
    // stream.write_all(&data).await.expect("Failed to write data");

    let out = child
        .wait_with_output()
        .expect("Failed to wait on firecracker process");

    println!("Firecracker process exited with status: {}", out.status);
}
