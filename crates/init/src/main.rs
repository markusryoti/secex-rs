use std::path::Path;
use std::time::Duration;

use nix::mount::{MsFlags, mount};
use nix::sys::reboot::{RebootMode, reboot};
// use tokio_vsock::{VsockAddr, VsockStream};

// use nix::sys::socket::{AddressFamily, SockFlag, SockType, VsockAddr, connect, socket};
use std::os::unix::io::AsRawFd;
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

    let mut count = 10;

    let mut stream = loop {
        match VsockStream::connect(VsockAddr::new(2, 5000)).await {
            Ok(s) => break s,
            Err(_) => {
                if count >= 10 {
                    panic!("Failed to connect to orchestrator after multiple attempts");
                }
                println!(
                    "Waiting for orchestrator to be ready... ({} attempts left)",
                    10 - count
                );
                tokio::time::sleep(Duration::from_millis(200)).await;
                count += 1;
            }
        }
    };

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
