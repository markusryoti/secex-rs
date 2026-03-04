use std::{collections::HashMap, io::Write, process::Command};

use nix::sys::reboot::{RebootMode, reboot};
use tokio_vsock::{VsockAddr, VsockListener, VsockStream};
use tracing::{error, info};

mod messaging;
mod mounts;

fn shutdown_actions() {
    // Flush all file system buffers to ensure data integrity before rebooting
    nix::unistd::sync();

    // This tells the kernel to power down the system
    reboot(RebootMode::RB_AUTOBOOT).expect("Power off failed");
}

fn setup_networking(params: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
    let ip = params.get("vm.ip").ok_or("missing vm.ip")?;
    let gateway = params.get("vm.gateway").ok_or("missing vm.gateway")?;
    let iface = params
        .get("vm.iface")
        .unwrap_or(&"eth0".to_string())
        .clone();

    // Bring up loopback
    let status = Command::new("/sbin/ip")
        .args(["link", "set", "lo", "up"])
        .status()?;

    if !status.success() {
        return Err(format!("Failed to bring up loopback: {}", status).into());
    }
    info!("Loopback interface up");

    // Bring up eth0
    let status = Command::new("/sbin/ip")
        .args(["link", "set", "eth0", "up"])
        .status()?;

    if !status.success() {
        return Err(format!("Failed to bring up eth0: {}", status).into());
    }
    info!("eth0 interface up");

    // Assign IP address to eth0
    let status = Command::new("/sbin/ip")
        .args(["addr", "add", &format!("{}/30", ip), "dev", &iface])
        .status()?;

    if !status.success() {
        return Err(format!("Failed to assign IP: {}", status).into());
    }
    info!("IP address assigned");

    // Add default route
    let status = Command::new("/sbin/ip")
        .args(["route", "add", "default", "via", gateway, "dev", "eth0"])
        .status()?;

    if !status.success() {
        return Err(format!("Failed to add route: {}", status).into());
    }
    info!("Route added successfully");

    // Set DNS
    std::fs::write("/etc/resolv.conf", "nameserver 8.8.8.8\n")?;
    info!("DNS configured successfully");

    Ok(())
}

fn close_stream(mut stream: VsockStream) {
    let _ = stream.flush();
    let _ = stream.shutdown(std::net::Shutdown::Both);
}

fn parse_cmdline() -> HashMap<String, String> {
    let cmdline = std::fs::read_to_string("/proc/cmdline").unwrap_or_default();
    cmdline
        .split_whitespace()
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_ansi(false).init();

    info!("Init started. Checking mounts...");

    mounts::mount_drives();

    let params = parse_cmdline();

    info!("Mounts complete. Entering main loop.");

    match setup_networking(&params) {
        Ok(_) => (),
        Err(e) => {
            error!("Error setting up networking: {}", e);
            return;
        }
    }

    let cid: u32 = params
        .get("vm.cid")
        .and_then(|v| v.parse().ok())
        .expect("missing vm.cid");

    if !std::path::Path::new("/dev/vsock").exists() {
        info!("ERROR: /dev/vsock does not exist! Make sure the driver is loaded.");
    }

    let listener = match VsockListener::bind(VsockAddr::new(cid, 5001)) {
        Ok(l) => l,
        Err(e) => {
            info!("Failed to bind vsock listener: {}", e);
            shutdown_actions();
            return;
        }
    };

    info!("Init started. Listening on cid 3, port 5001...");

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

    close_stream(stream);
    shutdown_actions();
}
