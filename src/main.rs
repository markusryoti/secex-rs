use std::{net::Ipv4Addr, path::PathBuf, process::Command};

use macaddr::MacAddr;
use secure_exec_rs::FirecrackerConfig;

mod network;

struct VmConfig {
    id: String,
    api_socket: PathBuf,
    tap: String,
    host_ip: Ipv4Addr,
    guest_ip: Ipv4Addr,
    mac: MacAddr,
}

fn edit_vm_config(current_dir: &PathBuf) {
    let mut config = FirecrackerConfig::from_file(&current_dir.join("vm_config_template.json"))
        .expect("Failed to read Firecracker config file");

    config.fill_values(
        current_dir
            .join("vmlinux-6.1.155")
            .to_str()
            .expect("Invalid kernel path"),
        current_dir
            .join("ubuntu-24.04.ext4")
            .to_str()
            .expect("Invalid rootfs path"),
        "tap0",
        "06:00:AC:10:00:02",
        current_dir
            .join("firecracker.log")
            .to_str()
            .expect("Invalid log path"),
    );

    config
        .to_file(&current_dir.join("vm_config.json"))
        .expect("Failed to write Firecracker config file");

    println!("Wrote Firecracker config to vm_config.json",);
}

fn remove_existing_socket(socket_path: &str) {
    Command::new("sudo")
        .arg("rm")
        .arg("-f")
        .arg(socket_path)
        .status()
        .expect("Failed to remove socket with sudo");

    println!("Removed existing socket at {}", socket_path);
}

fn setup_vm_network(tap_ip: Ipv4Addr) {
    network::set_network_interface(&tap_ip);
    println!("Set up VM network with TAP device at {}", tap_ip);
}

fn main() {
    let socket_path = "/tmp/firecracker.socket";

    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    println!("Current directory: {}", current_dir.display());

    edit_vm_config(&current_dir);
    remove_existing_socket(socket_path);
    setup_vm_network(Ipv4Addr::new(172, 16, 0, 1));

    let firecracker_path = current_dir.join("firecracker");
    let child = Command::new("sudo")
        .arg(&firecracker_path)
        .arg("--api-sock")
        .arg(socket_path)
        .arg("--enable-pci")
        .arg("--config-file")
        .arg(current_dir.join("vm_config.json"))
        .spawn()
        .expect("Failed to start firecracker");

    println!("Firecracker started with PID: {}", child.id());

    // let key_path = current_dir.join("ubuntu-24.04.id_rsa");
    // configure_guest_network("172.16.0.2", "172.16.0.1", key_path.to_str().unwrap());

    child
        .wait_with_output()
        .expect("Failed to wait on firecracker process");
}
