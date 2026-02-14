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

// pub fn set_network_interface(tap_ip: &std::net::Ipv4Addr, tap_dev: &str) {
//     // Try to delete the TAP device, but don't panic if it doesn't exist
//     let _ = Command::new("sudo")
//         .arg("ip")
//         .arg("link")
//         .arg("del")
//         .arg(tap_dev)
//         .status();

//     Command::new("sudo")
//         .arg("ip")
//         .arg("tuntap")
//         .arg("add")
//         .arg("dev")
//         .arg(tap_dev)
//         .arg("mode")
//         .arg("tap")
//         .status()
//         .expect("Failed to add TAP device");

//     Command::new("sudo")
//         .arg("ip")
//         .arg("addr")
//         .arg("add")
//         .arg(format!("{}/30", tap_ip.to_string()))
//         .arg("dev")
//         .arg(tap_dev)
//         .status()
//         .expect("Failed to add address to TAP device");

//     Command::new("sudo")
//         .arg("ip")
//         .arg("link")
//         .arg("set")
//         .arg("dev")
//         .arg(tap_dev)
//         .arg("up")
//         .status()
//         .expect("Failed to set TAP device up");

//     Command::new("sudo")
//         .arg("sh")
//         .arg("-c")
//         .arg("echo 1 > /proc/sys/net/ipv4/ip_forward")
//         .status()
//         .expect("Failed to enable IP forwarding");

//     Command::new("sudo")
//         .arg("iptables")
//         .arg("-P")
//         .arg("FORWARD")
//         .arg("ACCEPT")
//         .status()
//         .expect("Failed to set iptables FORWARD policy to ACCEPT");

//     let host_iface_output = Command::new("ip")
//         .arg("-j")
//         .arg("route")
//         .arg("list")
//         .arg("default")
//         .output()
//         .expect("Failed to get default route");

//     let host_iface_json: serde_json::Value = serde_json::from_slice(&host_iface_output.stdout)
//         .expect("Failed to parse JSON from ip route output");

//     let host_iface = host_iface_json[0]["dev"]
//         .as_str()
//         .expect("Failed to get host interface name");

//     info!("Host interface for NAT: {}", host_iface);

//     Command::new("sudo")
//         .arg("iptables")
//         .arg("-t")
//         .arg("nat")
//         .arg("-D")
//         .arg("POSTROUTING")
//         .arg("-o")
//         .arg(host_iface)
//         .arg("-j")
//         .arg("MASQUERADE")
//         .status()
//         .ok(); // Ignore error if rule doesn't exist

//     Command::new("sudo")
//         .arg("iptables")
//         .arg("-t")
//         .arg("nat")
//         .arg("-A")
//         .arg("POSTROUTING")
//         .arg("-o")
//         .arg(host_iface)
//         .arg("-j")
//         .arg("MASQUERADE")
//         .status()
//         .expect("Failed to add iptables MASQUERADE rule");
// }
