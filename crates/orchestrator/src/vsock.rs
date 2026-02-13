use std::time::Duration;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};
use tracing::{debug, error, info};

pub async fn wait_for_socket(vsock_uds_path: &str) {
    while !std::path::Path::new(&vsock_uds_path).exists() {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    info!(
        "Unix socket {} is now available. Attempting to connect...",
        vsock_uds_path
    );
}

pub fn remove_existing_vsock(vsock_uds_path: &str) {
    match std::fs::remove_file(&vsock_uds_path) {
        Ok(_) => info!("Removed existing vsock UDS at {}", vsock_uds_path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            info!("No existing vsock UDS at {}, proceeding...", vsock_uds_path)
        }
        Err(e) => panic!("Failed to remove existing vsock UDS: {}", e),
    }
}

pub async fn connect_to_vsock(vsock_uds_path: &str) -> UnixStream {
    info!(
        "Connecting to guest via Unix socket at {}...",
        vsock_uds_path
    );

    let stream = loop {
        let mut s = match UnixStream::connect(vsock_uds_path).await {
            Ok(s) => {
                debug!("Successfully connected to vsock UDS at {}", vsock_uds_path);
                s
            }
            Err(e) => {
                error!("Failed to connect to vsock UDS: {}", e);
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };

        // Send the handshake immediately
        if let Err(_) = s.write_all(b"CONNECT 5001\n").await {
            continue;
        }

        // Read response - if we get "OK", we are truly connected to the guest
        let mut buf = [0u8; 32];
        match s.read(&mut buf).await {
            Ok(n) if n > 0 => {
                let resp = String::from_utf8_lossy(&buf[..n]);
                if resp.contains("OK") {
                    info!("Guest is ready and handshake successful!");
                    break s;
                }
            }
            Err(err) => {
                error!("Error during handshake: {}", err);
                continue;
            }
            Ok(_) => {
                debug!("Received empty response during handshake");
                continue;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };

    info!("Connected to guest via vsock UDS at {}", vsock_uds_path);

    stream
}
