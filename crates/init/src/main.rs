use std::path::Path;
use std::time::Duration;

use nix::mount::{MsFlags, mount};
use nix::sys::reboot::{RebootMode, reboot};
use tokio_vsock::{VsockAddr, VsockStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use protocol;

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

#[tokio::main]
async fn main() {
    println!("Init started. Checking mounts...");

    mount_drives();

    println!("Mounts complete. Entering main loop.");

    // let fd = socket(
    //     AddressFamily::Vsock,
    //     SockType::Stream,
    //     SockFlag::empty(),
    //     None,
    // )
    // .expect("Create fd");

    // // CID 2 is always the Host. Port 1234 (or whatever you choose).
    // let addr = VsockAddr::new(2, 1234);

    // println!("Guest connecting to host...");
    // connect(fd.as_raw_fd(), &addr).expect("connect");
    // println!("Connected!");

    // // Send data to host
    // nix::unistd::write(fd.as_raw_fd(), b"Hello from the Guest!").expect("to write");

    let mut count = 0;
    const MAX_ATTEMPTS: u32 = 30;

    println!("Attempting to connect to orchestrator at CID 2, port 5001...");

    let mut stream = loop {
        // Guest (CID 3) connects to host (CID 2) on port 5001
        // This connection gets bridged to the Unix socket /tmp/vsock-vm-1.sock
        match VsockStream::connect(VsockAddr::new(2, 5001)).await {
            Ok(s) => {
                println!("Successfully connected to orchestrator!");
                break s;
            }
            Err(e) => {
                if count >= MAX_ATTEMPTS {
                    eprintln!("Error on last attempt: {}", e);
                    panic!("Failed to connect to orchestrator after {} attempts", MAX_ATTEMPTS);
                }
                println!(
                    "Connection attempt {} failed: {}. Retrying... ({} attempts left)",
                    count + 1, e,
                    MAX_ATTEMPTS - count
                );
                tokio::time::sleep(Duration::from_millis(200)).await;
                count += 1;
            }
        }
    };

    println!("Connected to orchestrator");
    
    // Send Hello message to orchestrator
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

    println!("Sent Hello to orchestrator");

    // Wait for orchestrator's response
    let mut buffer = vec![0u8; 4];
    match tokio::time::timeout(Duration::from_secs(10), stream.read_exact(&mut buffer)).await {
        Ok(Ok(_)) => {
            let len = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
            let mut msg_buffer = vec![0u8; len];
            stream.read_exact(&mut msg_buffer).await.expect("Failed to read message");
            println!("Received response from orchestrator");
        }
        Ok(Err(e)) => eprintln!("Error reading orchestrator response: {}", e),
        Err(_) => eprintln!("Timeout waiting for orchestrator response"),
    }

    loop {
        let msg = protocol::recv_msg::<protocol::Envelope>(&mut stream)
            .expect("Failed to receive message");

        match msg.message {
            protocol::Message::Hello => {
                println!("Received Hello message from orchestrator");
                break;
            }
            protocol::Message::RunCommand(run_command) => {
                println!("Received RunCommand: {:?}", run_command);
            }
            protocol::Message::Stdout(stream_chunk) => todo!(),
            protocol::Message::Stderr(stream_chunk) => todo!(),
            protocol::Message::Exit(exit_status) => todo!(),
            protocol::Message::Cancel(cancel) => todo!(),
            protocol::Message::Shutdown => {
                println!("Received shutdown message. Exiting main loop.");
                break;
            }
        }
    }

    // std::thread::sleep(std::time::Duration::from_secs(10));

    println!("Work finished. Shutting down...");

    // Flush all file system buffers to ensure data integrity before rebooting
    nix::unistd::sync();

    // This tells the kernel to power down the system
    reboot(RebootMode::RB_AUTOBOOT).expect("Power off failed");
}
