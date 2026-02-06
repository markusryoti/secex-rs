use protocol;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio_vsock::{VsockAddr, VsockListener};

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

    tokio::time::sleep(Duration::from_secs(5)).await;

    let listener = VsockListener::bind(VsockAddr::new(2, 5000)).expect("Vsock listen failed");
    let (mut stream, _) = listener
        .accept()
        .await
        .expect("Failed to accept vsock connection");

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

    let out = child
        .wait_with_output()
        .expect("Failed to wait on firecracker process");

    println!("Firecracker process exited with status: {}", out.status);
}
