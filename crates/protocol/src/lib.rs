pub mod tar;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Serialize, Deserialize)]
pub struct Envelope {
    pub version: u32,
    pub message: Message,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    Hello,
    RunCommand(RunCommand),
    CommandOutput(CommandOutput),
    Shutdown,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommandOutput {
    pub output: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RunCommand {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: Option<String>,
}

pub async fn send_msg(
    stream: &mut (impl AsyncWriteExt + std::marker::Unpin),
    msg: Message,
) -> Result<(), Box<dyn std::error::Error>> {
    let env = Envelope {
        version: 1,
        message: msg,
    };

    let data = serde_json::to_vec(&env)?;

    stream.write_all(&(data.len() as u32).to_be_bytes()).await?;
    stream.write_all(&data).await?;

    Ok(())
}

pub async fn recv_msg(
    stream: &mut (impl AsyncReadExt + std::marker::Unpin),
) -> Result<Message, Box<dyn std::error::Error>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;

    let len = u32::from_be_bytes(len_buf) as usize;
    let mut msg_buf = vec![0u8; len];

    stream.read_exact(&mut msg_buf).await?;

    let envelope: Envelope = serde_json::from_slice(&msg_buf)?;

    Ok(envelope.message)
}
