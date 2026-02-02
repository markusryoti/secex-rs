use std::{net::Ipv4Addr, path::PathBuf, process::Command};

use macaddr::{MacAddr, MacAddr6};
use secure_exec_rs::FirecrackerConfig;

mod network;

const VMS: Vec<VmConfig> = Vec::new();

struct VmConfig {
    id: String,
    api_socket: PathBuf,
    tap: String,
    host_ip: Ipv4Addr,
    guest_ip: Ipv4Addr,
    mac: MacAddr,
}

impl VmConfig {
    pub fn new() -> Self {
        let sequence_num = VMS.len() + 1;
        let id = format!("vm-{}", sequence_num);
        let socket_name = format!("/tmp/firecracker-{}.socket", sequence_num);
        let tap = format!("tap{}", sequence_num);

        VmConfig {
            id: id,
            api_socket: PathBuf::from(socket_name),
            tap: tap,
            host_ip: Ipv4Addr::new(172, 16, 0, 1),
            guest_ip: Ipv4Addr::new(172, 16, 0, 2),
            mac: MacAddr6::new(0x06, 0x00, 0xAC, 0x10, 0x00, 0x02).into(),
        }
    }
}

fn edit_vm_config(current_dir: &PathBuf, vm_config: &VmConfig) {
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
        vm_config.tap.as_str(),
        vm_config.mac.to_string().as_str(),
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

fn setup_vm_network(tap_ip: &Ipv4Addr) {
    network::set_network_interface(&tap_ip);
    println!("Set up VM network with TAP device at {}", tap_ip);
}

fn main() {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    println!("Current directory: {}", current_dir.display());

    let vm = VmConfig::new();

    edit_vm_config(&current_dir, &vm);
    remove_existing_socket(vm.api_socket.to_str().unwrap());
    setup_vm_network(&vm.host_ip);

    let firecracker_path = current_dir.join("firecracker");
    let child = Command::new("sudo")
        .arg(&firecracker_path)
        .arg("--api-sock")
        .arg(vm.api_socket.to_str().unwrap())
        .arg("--enable-pci")
        .arg("--config-file")
        .arg(current_dir.join("vm_config.json"))
        .spawn()
        .expect("Failed to start firecracker");

    println!("Firecracker started with PID: {}", child.id());

    child
        .wait_with_output()
        .expect("Failed to wait on firecracker process");
}
