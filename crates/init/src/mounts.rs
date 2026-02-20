use std::path::Path;

use nix::mount::{MsFlags, mount};
use tracing::info;

pub fn mount_drives() {
    if !Path::new("/dev/null").exists() {
        match mount(
            Some("devtmpfs"),
            "/dev",
            Some("devtmpfs"),
            MsFlags::empty(),
            None::<&str>,
        ) {
            Ok(_) => info!("Mounted devtmpfs"),
            Err(nix::errno::Errno::EBUSY) => info!("/dev already mounted"),
            Err(e) => panic!("Failed to mount /dev: {}", e),
        }
    }

    match mount(
        Some("proc"),
        "/proc",
        Some("proc"),
        MsFlags::empty(),
        None::<&str>,
    ) {
        Ok(_) => info!("Mounted proc"),
        Err(_) => panic!("Failed to mount proc"),
    };

    match mount(
        Some("sysfs"),
        "/sys",
        Some("sysfs"),
        MsFlags::empty(),
        None::<&str>,
    ) {
        Ok(_) => info!("Mounted sysfs"),
        Err(_) => panic!("Failed to mount sysfs"),
    };
}
