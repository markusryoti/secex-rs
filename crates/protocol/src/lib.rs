use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Envelope {
    pub version: u32,
    pub message: Message,
}

#[derive(Serialize, Deserialize)]
pub enum Message {
    RunCommand(RunCommand),
    Stdout(StreamChunk),
    Stderr(StreamChunk),
    Exit(ExitStatus),
    Cancel(Cancel),
    Shutdown,
}

#[derive(Serialize, Deserialize)]
pub struct RunCommand {
    pub id: String,
    pub command: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct StreamChunk {
    pub id: String,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct ExitStatus {
    pub id: String,
    pub code: i32,
}

#[derive(Serialize, Deserialize)]
pub struct Cancel {
    pub id: String,
}
