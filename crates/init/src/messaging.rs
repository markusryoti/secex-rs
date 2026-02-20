use std::{os::unix::fs::PermissionsExt, path::Path, process::Output};

use protocol::{RunCommand, WorkspaceRunOptions};
use tokio_vsock::VsockStream;
use tracing::{error, info};

pub async fn handle_messages(stream: &mut VsockStream) {
    loop {
        let message = protocol::recv_msg(stream)
            .await
            .expect("Failed to receive message");

        match message {
            protocol::Message::Hello => {
                info!("Orchestrator said Hello! Sending response...");

                match protocol::send_msg(stream, protocol::Message::Hello).await {
                    Ok(_) => continue,
                    Err(e) => {
                        error!("Error responding to hello message: {}", e);
                        continue;
                    }
                }
            }
            protocol::Message::RunCommand(cmd) => {
                info!("Received RunCommand: {}", cmd.command);
                match handle_run_individual_command(stream, cmd).await {
                    Ok(_) => continue,
                    Err(e) => {
                        error!("Error running command: {}", e);
                        continue;
                    }
                }
            }
            protocol::Message::RunWorkspace(wo) => match handle_run_workspace(stream, wo).await {
                Ok(_) => continue,
                Err(e) => {
                    error!("Error running workspace: {}", e);
                    continue;
                }
            },
            protocol::Message::Shutdown => {
                info!("Shutting down guest...");
                return;
            }
            _ => info!("Received other message"),
        }
    }
}

async fn handle_run_individual_command(
    stream: &mut VsockStream,
    cmd: RunCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    let child = tokio::process::Command::new(&cmd.command)
        .args(&cmd.args)
        .envs(&cmd.env)
        .current_dir(cmd.working_dir.unwrap_or_else(|| "/".to_string()))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let out = child.wait_with_output().await?;

    info!("Command exited with status: {}", out.status);

    let stdout_str = String::from_utf8_lossy(&out.stdout).trim().to_string();

    info!("Command out: {:?}", stdout_str);

    protocol::send_msg(
        stream,
        protocol::Message::CommandOutput(protocol::CommandOutput {
            output: stdout_str.to_string(),
        }),
    )
    .await?;

    Ok(())
}

async fn handle_run_workspace(
    stream: &mut VsockStream,
    wo: WorkspaceRunOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Received file transfer of {} bytes", wo.data.len());

    let workspace = "/tmp/workspace";

    match save_upload_payload(workspace, &wo.data) {
        Ok(_) => info!("Workspace directory ready"),
        Err(e) => {
            error!("Error creating workspace: {}", e);
            return Err(e);
        }
    }

    let entrypoint = Path::join(Path::new(workspace), &wo.entrypoint);

    info!("Entry point defined: {:?}", entrypoint);

    match make_entrypoint_executable(&entrypoint) {
        Ok(_) => info!("Entrypoint converted to executable"),
        Err(e) => {
            error!("Failed to make entrypoint executable: {}", e);
            return Err(e);
        }
    }

    let out = match run_program(&entrypoint, workspace).await {
        Ok(o) => o,
        Err(e) => {
            error!("Error running program: {}", e);
            return Err(e);
        }
    };

    match out.status.success() {
        true => info!("Process completed successfully"),
        false => {
            error!("Process exited with status: {}", out.status);
            let msg = format!("Process exited with status: {}", out.status).to_string();
            return Err(msg.into());
        }
    }

    let stdout_str = String::from_utf8_lossy(&out.stdout).trim().to_string();

    info!("Command stdout: {}", stdout_str);

    match protocol::send_msg(
        stream,
        protocol::Message::CommandOutput(protocol::CommandOutput {
            output: stdout_str.to_string(),
        }),
    )
    .await
    {
        Ok(_) => info!("Command output sent"),
        Err(e) => {
            error!("Error sending command output: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

fn save_upload_payload(workspace: &str, data: &Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    if std::path::Path::new(workspace).exists() {
        std::fs::remove_dir_all(workspace)?;
    }

    std::fs::create_dir_all(workspace)?;

    let tar_path = format!("{}/code.tar", workspace);
    std::fs::write(&tar_path, data)?;

    protocol::tar::untar_workspace(&tar_path, workspace)?;

    Ok(())
}

fn make_entrypoint_executable(entrypoint: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let meta = std::fs::metadata(entrypoint)?;
    let mut perms = meta.permissions();
    perms.set_mode(0o755);

    std::fs::set_permissions(entrypoint, perms)?;

    Ok(())
}

async fn run_program(
    entrypoint: &Path,
    workspace: &str,
) -> Result<Output, Box<dyn std::error::Error>> {
    let child = tokio::process::Command::new("/bin/sh")
        .arg(entrypoint)
        .current_dir(workspace)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let out = child.wait_with_output().await?;

    Ok(out)
}
