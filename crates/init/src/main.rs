use std::path::Path;

use nix::mount::{MsFlags, mount};
use nix::sys::reboot::{RebootMode, reboot};
use tokio::net::UnixStream;
use tokio_vsock::{VsockAddr, VsockStream};

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

    let mut stream = VsockStream::connect(VsockAddr::new(2, 5000))
        .await
        .expect("Failed to connect to orchestrator vsock");

    println!("Connected to orchestrator");

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
