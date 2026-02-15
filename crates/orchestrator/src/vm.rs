use std::{
    fs::File,
    net::Ipv4Addr,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

use macaddr::{MacAddr, MacAddr6};
use tokio::{io::AsyncReadExt, net::unix::OwnedWriteHalf, process::Child};
use tracing::{error, info};

use crate::{firecracker, network, vsock};

pub struct VmStore {
    vms: Vec<Arc<VmConfig>>,
}

impl VmStore {
    pub fn new() -> Self {
        VmStore { vms: Vec::new() }
    }

    pub fn add_vm(&mut self, vm: Arc<VmConfig>) {
        self.vms.push(vm);
    }

    pub fn get_vm(&self, id: &str) -> Option<&Arc<VmConfig>> {
        let found = self.vms.iter().find(|vm| vm.id == id);
        found
    }

    pub fn remove_vm(&mut self, id: &str) {
        self.vms.retain(|vm| vm.id != id);
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
    process: Mutex<Option<Child>>,
    writer: Mutex<Option<OwnedWriteHalf>>,
    vsock_path: String,
}

impl VmConfig {
    pub fn new(seq: usize) -> Self {
        let id = format!("vm-{}", seq);
        let socket_name = format!("/tmp/firecracker-{}.socket", seq);
        let vsock_uds_path = format!("/tmp/vsock-{}.sock", id);

        let tap = format!("tap{}", seq);

        VmConfig {
            id: id,
            api_socket: PathBuf::from(socket_name),
            tap: tap,
            host_ip: Ipv4Addr::new(172, 16, 0, 1),
            mac: MacAddr6::new(0x06, 0x00, 0xAC, 0x10, 0x00, 0x02).into(),
            process: Mutex::new(None),
            writer: Mutex::new(None),
            vsock_path: vsock_uds_path,
        }
    }

    pub fn initialize(&self) {
        self.edit_vm_config(&self.vsock_path);
        self.remove_existing_socket();

        vsock::remove_existing_vsock(&self.vsock_path);

        network::setup_tap_device(&self.tap, &self.host_ip.to_string(), "/30")
            .expect("Tap setup failed");
    }

    pub async fn launch(&self) {
        let current_dir = std::env::current_dir().expect("Failed to get current directory");
        let firecracker_path = current_dir.join("firecracker");

        let stdout_file =
            File::create(format!("{}.out.log", self.id)).expect("Failed to create stdout log file");
        let stderr_file =
            File::create(format!("{}.err.log", self.id)).expect("Failed to create stderr log file");

        let child = tokio::process::Command::new(&firecracker_path)
            .arg("--api-sock")
            .arg(self.api_socket.to_str().unwrap())
            .arg("--enable-pci")
            .arg("--config-file")
            .arg(current_dir.join(self.config_name()))
            .stdout(Stdio::from(stdout_file))
            .stderr(Stdio::from(stderr_file))
            .spawn()
            .expect("Failed to start firecracker");

        {
            let mut process = self.process.lock().expect("Failed to grab process mutex");
            *process = Some(child);
        }

        info!(
            "VM {} launched with API socket at {:?} and TAP device {}",
            self.id, self.api_socket, self.tap
        );
    }

    pub async fn connect(&self) {
        vsock::wait_for_socket(&self.vsock_path).await;

        let stream = vsock::connect_to_vsock(&self.vsock_path).await;

        let (reader, writer) = stream.into_split();

        {
            let mut write_guard = self.writer.lock().expect("Failed to get write mutex");
            *write_guard = Some(writer);
        }

        tokio::spawn(handle_incoming(reader));
    }

    pub async fn send_message(
        &self,
        msg: protocol::Message,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut write_guard = self.writer.lock().expect("Failed to grab writer mutex");
        let mut stream = write_guard.as_mut().unwrap();
        protocol::send_msg(&mut stream, msg).await
    }

    pub fn cleanup(&self) {
        network::cleanup_tap_device(&self.tap).expect("Failed to delete tap");
    }

    fn edit_vm_config(&self, vsock_uds_path: &str) {
        let current_dir = std::env::current_dir().expect("Failed to get current directory");

        let mut config = firecracker::FirecrackerConfig::from_file(
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

        info!("Wrote Firecracker config to {}", config_file.display());
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

        info!("Removed existing socket at {}", self.api_socket.display());
    }
}

async fn handle_incoming<T: AsyncReadExt + Unpin>(mut stream: T) {
    loop {
        let message = match protocol::recv_msg(&mut stream).await {
            Ok(m) => m,
            Err(e) => {
                error!("Error receiving message: {}", e);
                return;
            }
        };

        match message {
            protocol::Message::Hello => {
                info!("Guest said Hello!");
            }
            protocol::Message::CommandOutput(output) => {
                info!("Received command output from guest: {}", output.output);
            }
            m => info!("Received other message: {:?}", m),
        }
    }
}
