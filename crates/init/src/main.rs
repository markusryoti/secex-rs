use std::path::Path;

use nix::mount::{MsFlags, mount};
use nix::sys::reboot::{RebootMode, reboot};

fn main() {
    println!("Init started. Checking mounts...");

    // Mount devtmpfs only if not already mounted
    // We check for /dev/null as a sign of life
    if !Path::new("/dev/null").exists() {
        match mount(
            Some("devtmpfs"),
            "/dev",
            Some("devtmpfs"),
            MsFlags::empty(),
            None::<&str>,
        ) {
            Ok(_) => println!("Mounted devtmpfs"),
            Err(e) if e == nix::errno::Errno::EBUSY => println!("/dev already mounted"),
            Err(e) => panic!("Failed to mount /dev: {}", e),
        }
    }

    let _ = mount(
        Some("proc"),
        "/proc",
        Some("proc"),
        MsFlags::empty(),
        None::<&str>,
    );

    let _ = mount(
        Some("sysfs"),
        "/sys",
        Some("sysfs"),
        MsFlags::empty(),
        None::<&str>,
    );

    println!("Mounts complete. Entering main loop.");

    std::thread::sleep(std::time::Duration::from_secs(10));

    println!("Work finished. Shutting down...");

    nix::unistd::sync();

    // This tells the kernel to power down the system
    reboot(RebootMode::RB_AUTOBOOT).expect("Power off failed");
}
