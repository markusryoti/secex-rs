use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FirecrackerConfig {
    #[serde(rename = "boot-source")]
    pub boot_source: BootSource,
    pub drives: Vec<Drive>,
    #[serde(rename = "machine-config")]
    pub machine_config: MachineConfig,
    #[serde(rename = "cpu-config")]
    pub cpu_config: Value,
    pub balloon: Value,
    #[serde(rename = "network-interfaces")]
    pub network_interfaces: Vec<NetworkInterface>,
    pub vsock: Value,
    pub logger: Logger,
    pub metrics: Value,
    #[serde(rename = "mmds-config")]
    pub mmds_config: Value,
    pub entropy: Value,
    pub pmem: Vec<Value>,
    #[serde(rename = "memory-hotplug")]
    pub memory_hotplug: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BootSource {
    pub kernel_image_path: String,
    pub boot_args: String,
    pub initrd_path: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Drive {
    pub drive_id: String,
    pub partuuid: Value,
    pub is_root_device: bool,
    pub cache_type: String,
    pub is_read_only: bool,
    pub path_on_host: String,
    pub io_engine: String,
    pub rate_limiter: Value,
    pub socket: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MachineConfig {
    pub vcpu_count: i64,
    pub mem_size_mib: i64,
    pub smt: bool,
    pub track_dirty_pages: bool,
    pub huge_pages: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub iface_id: String,
    pub guest_mac: String,
    pub host_dev_name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Logger {
    pub log_path: String,
    pub level: String,
    pub show_level: bool,
    pub show_log_origin: bool,
}

impl FirecrackerConfig {
    pub fn from_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let file_content = std::fs::read_to_string(path)?;
        let config: FirecrackerConfig = serde_json::from_str(&file_content)?;
        Ok(config)
    }

    pub fn to_file(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let json_content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json_content)?;
        Ok(())
    }

    pub fn fill_values(
        &mut self,
        kernel_image_path: &str,
        drive_path: &str,
        tap_name: &str,
        mac_address: &str,
        log_path: &str,
    ) {
        self.boot_source.kernel_image_path = kernel_image_path.to_string();
        if let Some(drive) = self.drives.get_mut(0) {
            drive.path_on_host = drive_path.to_string();
        }
        if let Some(net_iface) = self.network_interfaces.get_mut(0) {
            net_iface.host_dev_name = tap_name.to_string();
            net_iface.guest_mac = mac_address.to_string();
        }
        self.logger.log_path = log_path.to_string();
    }
}
