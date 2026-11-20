pub mod cpu;
pub mod ecosystem;
pub mod gpu;
pub mod memory;
pub mod network;
pub mod platform;
pub mod storage;

use crate::engine::{format_bytes, now_hms};
use crate::models::SystemReport;
use std::collections::HashMap;
use wmi::{COMLibrary, Variant, WMIConnection};

pub fn var_str(row: &HashMap<String, Variant>, key: &str) -> Option<String> {
    match row.get(key) {
        Some(Variant::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Some(Variant::I4(v)) => Some(v.to_string()),
        Some(Variant::UI4(v)) => Some(v.to_string()),
        Some(Variant::I8(v)) => Some(v.to_string()),
        Some(Variant::UI8(v)) => Some(v.to_string()),
        _ => None,
    }
}

pub fn var_u64(row: &HashMap<String, Variant>, key: &str) -> Option<u64> {
    match row.get(key) {
        Some(Variant::UI8(v)) => Some(*v),
        Some(Variant::UI4(v)) => Some(*v as u64),
        Some(Variant::UI2(v)) => Some(*v as u64),
        Some(Variant::UI1(v)) => Some(*v as u64),
        Some(Variant::I8(v)) if *v >= 0 => Some(*v as u64),
        Some(Variant::I4(v)) if *v >= 0 => Some(*v as u64),
        Some(Variant::I2(v)) if *v >= 0 => Some(*v as u64),
        Some(Variant::String(s)) => s.trim().parse::<u64>().ok(),
        _ => None,
    }
}

pub fn var_u32(row: &HashMap<String, Variant>, key: &str) -> Option<u32> {
    var_u64(row, key).and_then(|v| u32::try_from(v).ok())
}

pub fn var_bool(row: &HashMap<String, Variant>, key: &str) -> Option<bool> {
    match row.get(key) {
        Some(Variant::Bool(b)) => Some(*b),
        Some(Variant::UI1(v)) => Some(*v != 0),
        Some(Variant::I4(v)) => Some(*v != 0),
        Some(Variant::UI4(v)) => Some(*v != 0),
        _ => None,
    }
}

impl SystemReport {
    pub fn collect() -> Self {
        let mut diagnostics = Vec::new();
        let com = COMLibrary::new().ok();

        let wmi_cimv2 = com
            .as_ref()
            .and_then(|c| match WMIConnection::new(*c) {
                Ok(conn) => Some(conn),
                Err(e) => {
                    diagnostics.push(format!("WMI CIMV2 unavailable: {e}"));
                    None
                }
            });

        let wmi_storage = com.as_ref().and_then(|c| {
            match WMIConnection::with_namespace_path(r"ROOT\Microsoft\Windows\Storage", *c) {
                Ok(conn) => Some(conn),
                Err(e) => {
                    diagnostics.push(format!("WMI Storage namespace unavailable: {e}"));
                    None
                }
            }
        });

        let wmi_tpm = com.as_ref().and_then(|c| {
            WMIConnection::with_namespace_path(r"ROOT\CIMV2\Security\MicrosoftTpm", *c).ok()
        });

        let cpu = cpu::detect_cpu(wmi_cimv2.as_ref(), &mut diagnostics);
        let (gpus, displays, hags_enabled) = gpu::detect_gpus_and_displays(&mut diagnostics);
        let memory = memory::detect_memory(wmi_cimv2.as_ref(), &mut diagnostics);
        let (physical_disks, volumes) =
            storage::detect_storage(wmi_storage.as_ref(), wmi_cimv2.as_ref(), &mut diagnostics);
        let platform = platform::detect_platform(wmi_cimv2.as_ref(), &mut diagnostics);
        let security =
            platform::detect_security(wmi_tpm.as_ref(), hags_enabled, &mut diagnostics);
        let power = platform::detect_power(&mut diagnostics);
        let network_adapters = network::detect_adapters(&mut diagnostics);
        let ecosystem = ecosystem::detect_ecosystem();

        SystemReport {
            cpu,
            gpus,
            displays,
            memory,
            physical_disks,
            volumes,
            platform,
            security,
            power,
            network_adapters,
            ecosystem,
            diagnostics,
            scanned_at: now_hms(),
        }
    }

    #[allow(dead_code)]
    pub fn primary_gpu_name(&self) -> &str {
        self.gpus
            .iter()
            .find(|g| !g.is_integrated)
            .or_else(|| self.gpus.first())
            .map(|g| g.name.as_str())
            .unwrap_or("Unknown GPU")
    }

    #[allow(dead_code)]
    pub fn primary_adapter(&self) -> Option<&crate::models::NetworkAdapterInfo> {
        self.network_adapters
            .iter()
            .find(|a| a.is_physical_primary)
            .or_else(|| self.network_adapters.first())
    }

    pub fn to_markdown_summary(&self) -> String {
        let mut s = String::new();
        s.push_str("# Optimizer by Raghav - Full System Identification Report\n\n");
        s.push_str(&format!("Scanned at: {}\n\n", self.scanned_at));
        s.push_str(&format!(
            "## Operating System\n- OS: {} {} (Build {}.{})\n- Arch: {} | Host: {} | User: {}\n- Pending Reboot: {}\n\n",
            self.platform.os_name,
            self.platform.display_version,
            self.platform.build_number,
            self.platform.ubr,
            self.platform.architecture,
            self.platform.computer_name,
            self.platform.user_name,
            if self.platform.pending_reboot { "YES" } else { "No" }
        ));
        s.push_str(&format!(
            "## CPU\n- Model: {}\n- Cores/Threads: {}C / {}T ({})\n- Cache: L1 {} KB | L2 {} KB | L3 {} KB\n\n",
            self.cpu.brand,
            self.cpu.physical_cores,
            self.cpu.logical_threads,
            self.cpu.topology_label,
            self.cpu.l1_kb,
            self.cpu.l2_kb,
            self.cpu.l3_kb
        ));
        s.push_str("## GPUs\n");
        for g in &self.gpus {
            s.push_str(&format!(
                "- {} | VRAM: {} | Driver: {} | {}\n",
                g.name,
                format_bytes(g.vram_bytes),
                g.driver_version,
                if g.is_integrated { "Integrated" } else { "Discrete" }
            ));
        }
        s.push_str("\n## Displays\n");
        for d in &self.displays {
            s.push_str(&format!(
                "- {} ({}) - {}x{} @ {} Hz{}\n",
                d.monitor_name,
                d.device_name,
                d.width,
                d.height,
                d.refresh_hz,
                if d.is_primary { " [Primary]" } else { "" }
            ));
        }
        s.push_str(&format!(
            "\n## Memory\n- {} ({}/{} slots, {} @ {} MT/s)\n",
            format_bytes(self.memory.total_phys_bytes),
            self.memory.used_slots,
            self.memory.total_slots,
            self.memory.memory_type,
            self.memory.speed_mts
        ));
        for d in &self.memory.dimms {
            s.push_str(&format!(
                "  - {}: {} {} {} ({} MT/s, {})\n",
                d.slot,
                format_bytes(d.capacity_bytes),
                d.manufacturer,
                d.memory_type,
                d.configured_mts,
                d.form_factor
            ));
        }
        s.push_str("\n## Storage\n");
        for disk in &self.physical_disks {
            s.push_str(&format!(
                "- {} | {} | Bus: {} | Health: {} | {}\n",
                disk.friendly_name,
                disk.media_type,
                disk.bus_type,
                disk.health_status,
                format_bytes(disk.size_bytes)
            ));
        }
        for vol in &self.volumes {
            s.push_str(&format!(
                "- Volume {} ({}) [{}] - {} free of {} ({:.1}% used)\n",
                vol.mount,
                vol.label,
                vol.filesystem,
                format_bytes(vol.free_bytes),
                format_bytes(vol.total_bytes),
                vol.used_pct
            ));
        }
        s.push_str(&format!(
            "\n## Platform, Power & Security\n- Motherboard: {} | BIOS: {} ({})\n- Chassis: {}\n- Power Plan: {} | Timer Res: {:.2} ms | Hibernation: {}\n- Secure Boot: {:?} | TPM: {} ({}) | VBS/HVCI: {}\n",
            self.platform.motherboard,
            self.platform.bios_version,
            self.platform.bios_date,
            self.platform.chassis_type,
            self.power.active_scheme_name,
            self.power.timer_res_current_ms,
            if self.power.hibernation_enabled {
                format!("Enabled ({})", format_bytes(self.power.hiberfil_bytes))
            } else {
                "Disabled".to_string()
            },
            self.security.secure_boot,
            if self.security.tpm_enabled { "Enabled" } else if self.security.tpm_present { "Present" } else { "Not Detected" },
            self.security.tpm_spec_version,
            if self.security.vbs_active { "Active" } else { "Off" }
        ));
        s.push_str("\n## Network Adapters\n");
        for nic in &self.network_adapters {
            s.push_str(&format!(
                "- {} ({}) - {} Mbps | IPv4: {} | DNS: {}{}\n",
                nic.friendly_name,
                nic.adapter_kind,
                nic.link_speed_mbps,
                nic.ipv4_addrs.join(", "),
                nic.dns_servers.join(", "),
                if nic.is_physical_primary { " [PRIMARY]" } else { "" }
            ));
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_readonly_system_identification_smoke_test() {
        let report = SystemReport::collect();
        println!("{}", report.to_markdown_summary());
        assert!(!report.cpu.brand.trim().is_empty(), "CPU brand must not be empty");
        assert!(report.cpu.logical_threads > 0, "CPU logical threads must be > 0");
        assert!(!report.gpus.is_empty(), "At least 1 GPU must be identified");
        assert!(report.memory.total_phys_bytes > 0, "Total RAM must be > 0");
        assert!(!report.physical_disks.is_empty(), "At least 1 physical disk must be identified");
        assert!(!report.volumes.is_empty(), "At least 1 volume must be identified");
        assert!(!report.platform.os_name.trim().is_empty(), "OS name must not be empty");
        assert!(!report.platform.motherboard.trim().is_empty(), "Motherboard must not be empty");
    }
}
