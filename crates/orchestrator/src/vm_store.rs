use std::sync::Arc;

use crate::vm::VmConfig;

pub struct VmStore {
    vms: Vec<Arc<VmConfig>>,
}

impl VmStore {
    pub fn new() -> Self {
        VmStore { vms: Vec::new() }
    }

    pub fn add_vm(&mut self, vm: Arc<VmConfig>) {
        self.vms.push(vm);
    }

    pub fn get_vm(&self, id: &str) -> Option<&Arc<VmConfig>> {
        let found = self.vms.iter().find(|vm| vm.id == id);
        found
    }

    pub fn remove_vm(&mut self, id: &str) {
        self.vms.retain(|vm| vm.id != id);
    }

    pub fn len(&self) -> usize {
        self.vms.len()
    }
}
