use std::{
    collections::HashMap,
    io::{Read, Write},
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Serialize, Deserialize)]
pub struct Envelope {
    pub version: u32,
    pub message: Message,
}

#[derive(Serialize, Deserialize)]
pub enum Message {
    Hello,
    RunCommand(RunCommand),
    Stdout(StreamChunk),
    Stderr(StreamChunk),
    Exit(ExitStatus),
    Cancel(Cancel),
    Shutdown,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RunCommand {
    pub id: String,
    pub command: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct StreamChunk {
    // pub id: String,
    // pub data: Vec<u8>,
    pub job_id: String,
    pub stream: StreamType,
    pub data: Vec<u8>,
    pub eof: bool,
}

#[derive(Serialize, Deserialize)]
pub enum StreamType {
    Stdin,
    Stdout,
    Stderr,
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

pub fn send_msg<T: Serialize>(
    stream: &mut impl Write,
    msg: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = serde_json::to_vec(msg)?;
    let len = (data.len() as u32).to_be_bytes();

    stream.write_all(&len)?;
    stream.write_all(&data)?;

    Ok(())
}

pub fn recv_msg<T: DeserializeOwned>(
    stream: &mut impl Read,
) -> Result<T, Box<dyn std::error::Error>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;

    let len = u32::from_be_bytes(len_buf) as usize;
    let mut buf = vec![0; len];

    stream.read_exact(&mut buf)?;

    Ok(serde_json::from_slice(&buf)?)
}
