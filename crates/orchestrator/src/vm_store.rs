use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

use crate::vm::VmConfig;

pub struct VmStore {
    vms: HashMap<String, Arc<Mutex<VmConfig>>>,
}

impl VmStore {
    pub fn new() -> Self {
        VmStore {
            vms: HashMap::new(),
        }
    }

    pub fn add_vm(&mut self, id: &str, vm: VmConfig) {
        self.vms.insert(id.into(), Arc::new(Mutex::new(vm)));
    }

    pub fn get_vm(&self, id: &str) -> Option<Arc<Mutex<VmConfig>>> {
        let vm = self.vms.get(&id.to_string()).cloned();
        vm
    }

    pub fn remove_vm(&mut self, id: &str) {
        self.vms.remove(&id.to_string());
    }

    pub fn len(&self) -> usize {
        self.vms.len()
    }
}
