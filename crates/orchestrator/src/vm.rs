use std::{
    fs::File,
    net::Ipv4Addr,
    path::PathBuf,
    process::{Command, Stdio},
};

use macaddr::{MacAddr, MacAddr6};
use tokio::{io::AsyncReadExt, net::unix::OwnedWriteHalf, process::Child};
use tracing::{error, info};

use crate::{firecracker, network, vsock};

pub struct VmHandle {
    pub id: String,
    tx: tokio::sync::mpsc::Sender<VmMessage>,
}

impl VmHandle {
    pub async fn start_vm(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.tx.send(VmMessage::StartVm).await?;

        Ok(())
    }

    pub async fn send_command(
        &self,
        cmd: protocol::RunCommand,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.tx.send(VmMessage::Command(cmd)).await?;

        Ok(())
    }

    pub async fn send_workspace_command(
        &self,
        cmd: protocol::WorkspaceRunOptions,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.tx.send(VmMessage::WorkspaceCommand(cmd)).await?;

        Ok(())
    }

    pub async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.tx.send(VmMessage::Shutdown).await?;

        Ok(())
    }
}

pub fn spawn_vm(seq: usize) -> VmHandle {
    let vm = VmActor::new(seq);
    let id = vm.id.clone();

    let (tx, rx) = tokio::sync::mpsc::channel(32);

    tokio::spawn(vm.run(rx));

    let handle = VmHandle { id, tx };

    handle
}

pub enum VmMessage {
    StartVm,
    Command(protocol::RunCommand),
    WorkspaceCommand(protocol::WorkspaceRunOptions),
    Shutdown,
}

pub struct VmActor {
    pub id: String,
    api_socket: PathBuf,
    tap: String,
    host_ip: Ipv4Addr,
    mac: MacAddr,
    process: std::sync::Mutex<Option<Child>>,
    writer: tokio::sync::Mutex<Option<OwnedWriteHalf>>,
    vsock_path: String,
}

impl VmActor {
    fn new(seq: usize) -> Self {
        let id = format!("vm-{}", seq);
        let socket_name = format!("/tmp/firecracker-{}.socket", seq);
        let vsock_uds_path = format!("/tmp/vsock-{}.sock", id);

        let tap = format!("tap{}", seq);

        VmActor {
            id: id,
            api_socket: PathBuf::from(socket_name),
            tap: tap,
            host_ip: Ipv4Addr::new(172, 16, 0, 1),
            mac: MacAddr6::new(0x06, 0x00, 0xAC, 0x10, 0x00, 0x02).into(),
            process: std::sync::Mutex::new(None),
            writer: tokio::sync::Mutex::new(None),
            vsock_path: vsock_uds_path,
        }
    }

    pub async fn launch(&self) {
        self.edit_vm_config(&self.vsock_path);
        self.remove_existing_socket();

        vsock::remove_existing_vsock(&self.vsock_path);

        network::setup_tap_device(&self.tap, &self.host_ip.to_string(), "/30")
            .expect("Tap setup failed");

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

        vsock::wait_for_socket(&self.vsock_path).await;

        let stream = vsock::connect_to_vsock(&self.vsock_path).await;

        let (reader, writer) = stream.into_split();

        {
            let mut write_guard = self.writer.lock().await;
            *write_guard = Some(writer);
        }

        tokio::spawn(handle_incoming(reader));
    }

    pub async fn run(self, mut rx: tokio::sync::mpsc::Receiver<VmMessage>) {
        while let Some(msg) = rx.recv().await {
            match msg {
                VmMessage::StartVm => self.launch().await,
                VmMessage::Command(run_command) => self
                    .send_message(protocol::Message::RunCommand(run_command))
                    .await
                    .unwrap(),
                VmMessage::WorkspaceCommand(workspace_run_options) => self
                    .send_message(protocol::Message::RunWorkspace(workspace_run_options))
                    .await
                    .unwrap(),
                VmMessage::Shutdown => self.cleanup(),
            }
        }
    }

    async fn send_message(&self, msg: protocol::Message) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(stream) = self.writer.lock().await.as_mut() {
            protocol::send_msg(stream, msg)
                .await
                .expect("Error sedning to stream");
        };

        Ok(())
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
                .join("vmlinux-kernel")
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
