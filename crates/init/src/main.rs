use std::path::Path;

use nix::mount::{MsFlags, mount};
use nix::sys::reboot::{RebootMode, reboot};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_vsock::{VsockAddr, VsockListener};

fn mount_drives() {
    if !Path::new("/dev/null").exists() {
        match mount(
            Some("devtmpfs"),
            "/dev",
            Some("devtmpfs"),
            MsFlags::empty(),
            None::<&str>,
        ) {
            Ok(_) => println!("Mounted devtmpfs"),
            Err(e) if e == nix::errno::Errno::EBUSY => println!("/dev already mounted"),
            Err(e) => panic!("Failed to mount /dev: {}", e),
        }
    }

    let _ = mount(
        Some("proc"),
        "/proc",
        Some("proc"),
        MsFlags::empty(),
        None::<&str>,
    );

    let _ = mount(
        Some("sysfs"),
        "/sys",
        Some("sysfs"),
        MsFlags::empty(),
        None::<&str>,
    );
}

fn shutdown_actions() {
    // Flush all file system buffers to ensure data integrity before rebooting
    nix::unistd::sync();

    // This tells the kernel to power down the system
    reboot(RebootMode::RB_AUTOBOOT).expect("Power off failed");
}

#[tokio::main]
async fn main() {
    println!("Init started. Checking mounts...");

    mount_drives();

    println!("Mounts complete. Entering main loop.");

    if !std::path::Path::new("/dev/vsock").exists() {
        println!("ERROR: /dev/vsock does not exist! Make sure the driver is loaded.");
    }

    println!("Init started. Listening on cid 3, port 5001...");

    let listener = match VsockListener::bind(VsockAddr::new(3, 5001)) {
        Ok(l) => l,
        Err(e) => {
            println!("Failed to bind vsock listener: {}", e);
            shutdown_actions();
            return;
        }
    };

    let (mut stream, addr) = match listener.accept().await {
        Ok((s, a)) => (s, a),
        Err(e) => {
            println!("Failed to accept vsock connection: {}", e);
            shutdown_actions();
            return;
        }
    };

    println!("Connection accepted from {:?}", addr);

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
                println!("Orchestrator said Hello! Sending response...");

                let response = protocol::Envelope {
                    version: 1,
                    message: protocol::Message::Hello, // Or a 'Ready' variant
                };
                let resp_data = serde_json::to_vec(&response).unwrap();

                stream
                    .write_all(&(resp_data.len() as u32).to_be_bytes())
                    .await
                    .unwrap();
                stream.write_all(&resp_data).await.unwrap();
                stream.flush().await.unwrap();
            }
            protocol::Message::Shutdown => {
                println!("Shutting down guest...");
                break;
            }
            _ => println!("Received other message"),
        }
    }

    shutdown_actions();
}
