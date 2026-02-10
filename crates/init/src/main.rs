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

    let mut buffer = vec![0u8; 4];

    match stream.read_exact(&mut buffer).await {
        Ok(_) => println!("Received data from vsock: {:?}", buffer),
        Err(e) => println!("Failed to read from vsock: {}", e),
    }

    shutdown_actions();
}
