use std::path::Path;

use nix::mount::{MsFlags, mount};
use nix::sys::reboot::{RebootMode, reboot};
use tokio::io::AsyncReadExt;
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

#[tokio::main]
async fn main() {
    println!("Init started. Checking mounts...");

    mount_drives();

    println!("Mounts complete. Entering main loop.");

    if !std::path::Path::new("/dev/vsock").exists() {
        println!("ERROR: /dev/vsock does not exist! Make sure the driver is loaded.");
    }

    println!("Init started. Listening on cid 2, port 5001...");

    let listener =
        VsockListener::bind(VsockAddr::new(2, 5001)).expect("Failed to bind vsock listener");

    let (mut stream, addr) = listener
        .accept()
        .await
        .expect("Failed to accept connection");

    println!("Connection accepted from {:?}", addr);

    let mut buffer = vec![0u8; 4];
    stream.read_exact(&mut buffer).await.unwrap();

    println!("Work finished. Shutting down...");

    // Flush all file system buffers to ensure data integrity before rebooting
    nix::unistd::sync();

    // This tells the kernel to power down the system
    reboot(RebootMode::RB_AUTOBOOT).expect("Power off failed");
}
