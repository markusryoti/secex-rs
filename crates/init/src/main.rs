use std::io::Write;

use nix::sys::reboot::{RebootMode, reboot};
use tokio_vsock::{VsockAddr, VsockListener};
use tracing::info;

mod messaging;
mod mounts;

fn shutdown_actions() {
    // Flush all file system buffers to ensure data integrity before rebooting
    nix::unistd::sync();

    // This tells the kernel to power down the system
    reboot(RebootMode::RB_AUTOBOOT).expect("Power off failed");
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_ansi(false).init();

    info!("Init started. Checking mounts...");

    mounts::mount_drives();

    info!("Mounts complete. Entering main loop.");

    if !std::path::Path::new("/dev/vsock").exists() {
        info!("ERROR: /dev/vsock does not exist! Make sure the driver is loaded.");
    }

    info!("Init started. Listening on cid 3, port 5001...");

    let listener = match VsockListener::bind(VsockAddr::new(3, 5001)) {
        Ok(l) => l,
        Err(e) => {
            info!("Failed to bind vsock listener: {}", e);
            shutdown_actions();
            return;
        }
    };

    let (mut stream, addr) = match listener.accept().await {
        Ok((s, a)) => (s, a),
        Err(e) => {
            info!("Failed to accept vsock connection: {}", e);
            shutdown_actions();
            return;
        }
    };

    info!("Connection accepted from {:?}", addr);

    messaging::handle_messages(&mut stream).await;

    let _ = stream.flush();
    let _ = stream.shutdown(std::net::Shutdown::Both);

    shutdown_actions();
}
