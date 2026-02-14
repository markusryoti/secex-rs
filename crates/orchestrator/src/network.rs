use std::process::Command;

use tracing::info;

pub fn setup_tap_device(
    tap_name: &str,
    tap_ip: &str,
    mask: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Setting up TAP device: {}", tap_name);

    // Delete existing TAP device if it exists
    Command::new("sudo")
        .args(["ip", "link", "del", tap_name])
        .output()
        .ok(); // Ignore errors if it doesn't exist

    // Create TAP device
    let status = Command::new("sudo")
        .args(["ip", "tuntap", "add", "dev", tap_name, "mode", "tap"])
        .status()?;

    if !status.success() {
        return Err(format!("Failed to create TAP device").into());
    }
    info!("TAP device created");

    // Assign IP address
    let status = Command::new("sudo")
        .args([
            "ip",
            "addr",
            "add",
            &format!("{}{}", tap_ip, mask),
            "dev",
            tap_name,
        ])
        .status()?;

    if !status.success() {
        return Err(format!("Failed to assign IP to TAP device").into());
    }
    info!("IP assigned to TAP device");

    // Bring up TAP device
    let status = Command::new("sudo")
        .args(["ip", "link", "set", "dev", tap_name, "up"])
        .status()?;

    if !status.success() {
        return Err(format!("Failed to bring up TAP device").into());
    }
    info!("TAP device is up");

    // Enable IP forwarding
    let status = Command::new("sudo")
        .args(["sh", "-c", "echo 1 > /proc/sys/net/ipv4/ip_forward"])
        .status()?;

    if !status.success() {
        return Err(format!("Failed to enable IP forwarding").into());
    }
    info!("IP forwarding enabled");

    // Set FORWARD policy
    Command::new("sudo")
        .args(["iptables", "-P", "FORWARD", "ACCEPT"])
        .status()?;

    // Get host network interface
    let output = Command::new("ip")
        .args(["-j", "route", "list", "default"])
        .output()?;

    let routes_json = String::from_utf8(output.stdout)?;
    let routes: serde_json::Value = serde_json::from_str(&routes_json)?;
    let host_iface = routes[0]["dev"]
        .as_str()
        .ok_or("Could not determine host interface")?;

    info!("Host interface: {}", host_iface);

    // Remove existing MASQUERADE rule if it exists
    Command::new("sudo")
        .args([
            "iptables",
            "-t",
            "nat",
            "-D",
            "POSTROUTING",
            "-o",
            host_iface,
            "-j",
            "MASQUERADE",
        ])
        .output()
        .ok(); // Ignore errors

    // Add MASQUERADE rule
    let status = Command::new("sudo")
        .args([
            "iptables",
            "-t",
            "nat",
            "-A",
            "POSTROUTING",
            "-o",
            host_iface,
            "-j",
            "MASQUERADE",
        ])
        .status()?;

    if !status.success() {
        return Err(format!("Failed to add MASQUERADE rule").into());
    }
    info!("MASQUERADE rule added");

    Ok(())
}

pub fn cleanup_tap_device(tap_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    Command::new("sudo")
        .args(["ip", "link", "del", tap_name])
        .status()?;

    info!("TAP device {} removed", tap_name);
    Ok(())
}
