use std::{net::Ipv4Addr, path::PathBuf, process::Command};

use macaddr::{MacAddr, MacAddr6};
use orchestrator::FirecrackerConfig;

use crate::network;

pub struct VmStore {
    vms: Vec<VmConfig>,
}

impl VmStore {
    pub fn new() -> Self {
        VmStore { vms: Vec::new() }
    }

    pub fn add_vm(&mut self, vm: VmConfig) {
        self.vms.push(vm);
    }

    pub fn get(&self, id: &str) -> Option<&VmConfig> {
        self.vms.iter().find(|vm| vm.id == id)
    }

    pub fn len(&self) -> usize {
        self.vms.len()
    }
}

pub struct VmConfig {
    pub id: String,
    api_socket: PathBuf,
    tap: String,
    host_ip: Ipv4Addr,
    mac: MacAddr,
}

impl VmConfig {
    pub fn new(seq: usize) -> Self {
        let id = format!("vm-{}", seq);
        let socket_name = format!("/tmp/firecracker-{}.socket", seq);
        let tap = format!("tap{}", seq);

        VmConfig {
            id: id,
            api_socket: PathBuf::from(socket_name),
            tap: tap,
            host_ip: Ipv4Addr::new(172, 16, 0, 1),
            mac: MacAddr6::new(0x06, 0x00, 0xAC, 0x10, 0x00, 0x02).into(),
        }
    }

    pub fn initialize(&self, vsock_uds_path: &str) {
        self.edit_vm_config(vsock_uds_path);
        self.remove_existing_socket();
        self.setup_vm_network();
    }

    pub fn launch(&self) -> std::process::Child {
        let current_dir = std::env::current_dir().expect("Failed to get current directory");
        let firecracker_path = current_dir.join("firecracker");

        let child = Command::new("sudo")
            .arg(&firecracker_path)
            .arg("--api-sock")
            .arg(self.api_socket.to_str().unwrap())
            .arg("--enable-pci")
            .arg("--config-file")
            .arg(current_dir.join(self.config_name()))
            .spawn()
            .expect("Failed to start firecracker");

        child
    }

    fn edit_vm_config(&self, vsock_uds_path: &str) {
        let current_dir = std::env::current_dir().expect("Failed to get current directory");

        let mut config = FirecrackerConfig::from_file(
            &current_dir.join("crates/orchestrator/vm_config_template.json"),
        )
        .expect("Failed to read Firecracker config file");

        let log_path = current_dir.join(format!("{}-firecracker.log", self.id));

        config.fill_values(
            current_dir
                .join("vmlinux-6.1.155")
                .to_str()
                .expect("Invalid kernel path"),
            current_dir
                .join("build/rootfs.ext4")
                .to_str()
                .expect("Invalid rootfs path"),
            &self.tap,
            &self.mac.to_string(),
            current_dir
                .join(log_path)
                .to_str()
                .expect("Invalid log path"),
            vsock_uds_path,
        );

        let config_file = current_dir.join(self.config_name());

        config
            .to_file(&config_file)
            .expect("Failed to write Firecracker config file");

        println!("Wrote Firecracker config to {}", config_file.display());
    }

    fn config_name(&self) -> String {
        format!("{}-vm_config.json", self.id)
    }

    fn remove_existing_socket(&self) {
        Command::new("sudo")
            .arg("rm")
            .arg("-f")
            .arg(self.api_socket.to_str().unwrap())
            .status()
            .expect("Failed to remove socket with sudo");

        println!("Removed existing socket at {}", self.api_socket.display());
    }

    fn setup_vm_network(&self) {
        network::set_network_interface(&self.host_ip, &self.tap);
        println!("Set up VM network with TAP device at {}", self.host_ip);
    }
}
