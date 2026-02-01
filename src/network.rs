use std::process::Command;

pub fn set_network_interface(tap_ip: &std::net::Ipv4Addr) {
    let tap_dev = "tap0";

    Command::new("sudo")
        .arg("ip")
        .arg("link")
        .arg("del")
        .arg(tap_dev)
        .status()
        .expect("Failed to delete existing TAP device");

    Command::new("sudo")
        .arg("ip")
        .arg("tuntap")
        .arg("add")
        .arg("dev")
        .arg(tap_dev)
        .arg("mode")
        .arg("tap")
        .status()
        .expect("Failed to add TAP device");

    Command::new("sudo")
        .arg("ip")
        .arg("addr")
        .arg("add")
        .arg(format!("{}/30", tap_ip.to_string()))
        .arg("dev")
        .arg(tap_dev)
        .status()
        .expect("Failed to add address to TAP device");

    Command::new("sudo")
        .arg("ip")
        .arg("link")
        .arg("set")
        .arg("dev")
        .arg(tap_dev)
        .arg("up")
        .status()
        .expect("Failed to set TAP device up");

    Command::new("sudo")
        .arg("sh")
        .arg("-c")
        .arg("echo 1 > /proc/sys/net/ipv4/ip_forward")
        .status()
        .expect("Failed to enable IP forwarding");

    Command::new("sudo")
        .arg("iptables")
        .arg("-P")
        .arg("FORWARD")
        .arg("ACCEPT")
        .status()
        .expect("Failed to set iptables FORWARD policy to ACCEPT");

    let host_iface_output = Command::new("ip")
        .arg("-j")
        .arg("route")
        .arg("list")
        .arg("default")
        .output()
        .expect("Failed to get default route");

    let host_iface_json: serde_json::Value = serde_json::from_slice(&host_iface_output.stdout)
        .expect("Failed to parse JSON from ip route output");

    let host_iface = host_iface_json[0]["dev"]
        .as_str()
        .expect("Failed to get host interface name");

    println!("Host interface for NAT: {}", host_iface);

    Command::new("sudo")
        .arg("iptables")
        .arg("-t")
        .arg("nat")
        .arg("-D")
        .arg("POSTROUTING")
        .arg("-o")
        .arg(host_iface)
        .arg("-j")
        .arg("MASQUERADE")
        .status()
        .ok(); // Ignore error if rule doesn't exist

    Command::new("sudo")
        .arg("iptables")
        .arg("-t")
        .arg("nat")
        .arg("-A")
        .arg("POSTROUTING")
        .arg("-o")
        .arg(host_iface)
        .arg("-j")
        .arg("MASQUERADE")
        .status()
        .expect("Failed to add iptables MASQUERADE rule");
}
