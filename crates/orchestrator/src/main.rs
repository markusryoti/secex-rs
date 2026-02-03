mod network;
mod vm;

fn main() {
    let store = &mut vm::VmStore::new();

    let vm = vm::VmConfig::new(store.len() + 1);
    let vm_id = vm.id.clone();

    store.add_vm(vm);

    let vm = store
        .get(&vm_id)
        .expect("Failed to retrieve VM configuration");

    vm.initialize();
    let child = vm.launch();

    println!("Firecracker started with PID: {}", child.id());

    child
        .wait_with_output()
        .expect("Failed to wait on firecracker process");
}
