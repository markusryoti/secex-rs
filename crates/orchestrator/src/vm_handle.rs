pub enum VmMessage {
    StartVm,
    Command(protocol::RunCommand),
    WorkspaceCommand(protocol::WorkspaceRunOptions),
    Shutdown,
}

pub struct VmHandle {
    pub id: String,
    tx: tokio::sync::mpsc::Sender<VmMessage>,
}

impl VmHandle {
    pub fn new(id: String, tx: tokio::sync::mpsc::Sender<VmMessage>) -> Self {
        VmHandle { id, tx }
    }

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
