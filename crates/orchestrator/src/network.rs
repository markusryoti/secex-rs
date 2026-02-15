use std::process::Command;

use tracing::info;

pub fn setup_ip_forwarding() -> Result<(), Box<dyn std::error::Error>> {
    info!("Setting up IP forwarding for VMs");

    // Enable IP forwarding
    let status = Command::new("sudo")
        .args(["sh", "-c", "echo 1 > /proc/sys/net/ipv4/ip_forward"])
        .status()?;

    if !status.success() {
        return Err("Failed to enable IP forwarding".into());
    }
    info!("IP forwarding enabled");

    // Set FORWARD policy to ACCEPT
    Command::new("sudo")
        .args(["iptables", "-P", "FORWARD", "ACCEPT"])
        .status()?;

    info!("iptables FORWARD policy set to ACCEPT");

    // Get host network interface
    let output = Command::new("ip")
        .args(["route", "show", "default"])
        .output()?;

    let route_str = String::from_utf8(output.stdout)?;
    let host_iface = route_str
        .split_whitespace()
        .skip_while(|&s| s != "dev")
        .nth(1)
        .ok_or("Could not determine host interface")?;

    info!("Host interface: {}", host_iface);

    // Remove existing MASQUERADE rule if it exists (ignore errors)
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
        .ok();

    // Add MASQUERADE rule for NAT
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
        return Err("Failed to add MASQUERADE rule".into());
    }
    info!("MASQUERADE rule added for internet access");

    Ok(())
}

pub fn cleanup_ip_forwarding() -> Result<(), Box<dyn std::error::Error>> {
    info!("Cleaning up IP forwarding rules");

    // Get host network interface
    let output = Command::new("ip")
        .args(["route", "show", "default"])
        .output()?;

    let route_str = String::from_utf8(output.stdout)?;
    let host_iface = route_str
        .split_whitespace()
        .skip_while(|&s| s != "dev")
        .nth(1)
        .ok_or("Could not determine host interface")?;

    // Remove MASQUERADE rule
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
        .status()
        .ok(); // Ignore errors if rule doesn't exist

    info!("MASQUERADE rule removed");

    // Optionally disable IP forwarding (be careful - might affect other services)
    // Command::new("sudo")
    //     .args(["sh", "-c", "echo 0 > /proc/sys/net/ipv4/ip_forward"])
    //     .status()?;

    Ok(())
}

pub fn setup_tap_device(
    tap_name: &str,
    tap_ip: &str,
    mask: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Delete if exists
    Command::new("sudo")
        .args(["ip", "link", "del", tap_name])
        .output()
        .ok();

    // Create TAP
    Command::new("sudo")
        .args(["ip", "tuntap", "add", "dev", tap_name, "mode", "tap"])
        .status()?;

    // Assign IP
    Command::new("sudo")
        .args([
            "ip",
            "addr",
            "add",
            &format!("{}{}", tap_ip, mask),
            "dev",
            tap_name,
        ])
        .status()?;

    // Bring up
    Command::new("sudo")
        .args(["ip", "link", "set", "dev", tap_name, "up"])
        .status()?;

    Ok(())
}

pub fn cleanup_tap_device(tap_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    Command::new("sudo")
        .args(["ip", "link", "del", tap_name])
        .status()?;

    info!("TAP device {} removed", tap_name);
    Ok(())
}
