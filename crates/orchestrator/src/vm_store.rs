use std::{collections::HashMap, sync::Arc};

use crate::vm_handle::VmHandle;

pub struct VmStore {
    vms: HashMap<String, Arc<VmHandle>>,
}

impl VmStore {
    pub fn new() -> Self {
        VmStore {
            vms: HashMap::new(),
        }
    }

    pub fn add_vm(&mut self, id: &str, vm: VmHandle) {
        self.vms.insert(id.into(), Arc::new(vm));
    }

    pub fn get_vm(&self, id: &str) -> Option<Arc<VmHandle>> {
        self.vms.get(id).cloned()
    }

    pub fn remove_vm(&mut self, id: &str) {
        self.vms.remove(id);
    }

    pub fn len(&self) -> usize {
        self.vms.len()
    }
}
