use std::process::Command;

fn remove_existing_socket() {
    Command::new("sudo")
        .arg("rm")
        .arg("-f")
        .arg("/tmp/firecracker.socket")
        .status()
        .expect("Failed to remove socket with sudo");
}

fn main() {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    println!("Current directory: {}", current_dir.display());

    remove_existing_socket();

    let firecracker_path = current_dir.join("firecracker");
    let child = Command::new(&firecracker_path)
        .current_dir(current_dir)
        .arg("--api-sock")
        .arg("/tmp/firecracker.socket")
        .arg("--enable-pci")
        .spawn()
        .expect("Failed to start firecracker");

    println!("firecracker started with PID: {}", child.id());

    child
        .wait_with_output()
        .expect("Failed to wait on firecracker process");
}
