use super::{var_str, var_u32, var_u64};
use crate::models::{DimmStick, MemoryInfo};
use std::collections::HashMap;
use windows::Win32::System::ProcessStatus::{GetPerformanceInfo, PERFORMANCE_INFORMATION};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use wmi::{Variant, WMIConnection};

pub fn detect_memory(wmi_con: Option<&WMIConnection>, diagnostics: &mut Vec<String>) -> MemoryInfo {
    let mut info = MemoryInfo::default();

    unsafe {
        let mut mem = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };
        if GlobalMemoryStatusEx(&mut mem).is_ok() {
            info.total_phys_bytes = mem.ullTotalPhys;
            info.avail_phys_bytes = mem.ullAvailPhys;
            info.used_phys_bytes = mem.ullTotalPhys.saturating_sub(mem.ullAvailPhys);
            info.memory_load_pct = mem.dwMemoryLoad;
        }

        let mut perf = PERFORMANCE_INFORMATION {
            cb: std::mem::size_of::<PERFORMANCE_INFORMATION>() as u32,
            ..Default::default()
        };
        if GetPerformanceInfo(&mut perf, perf.cb).is_ok() {
            let page_size = perf.PageSize as u64;
            info.commit_total_bytes = (perf.CommitTotal as u64).saturating_mul(page_size);
            info.commit_limit_bytes = (perf.CommitLimit as u64).saturating_mul(page_size);
            info.system_cache_bytes = (perf.SystemCache as u64).saturating_mul(page_size);
        }
    }

    if let Some(con) = wmi_con {
        match con.raw_query::<HashMap<String, Variant>>(
            "SELECT MemoryDevices FROM Win32_PhysicalMemoryArray",
        ) {
            Ok(rows) => {
                info.total_slots = rows
                    .iter()
                    .filter_map(|r| var_u32(r, "MemoryDevices"))
                    .sum();
            }
            Err(e) => diagnostics.push(format!("Win32_PhysicalMemoryArray query failed: {e}")),
        }

        match con.raw_query::<HashMap<String, Variant>>(
            "SELECT DeviceLocator, Manufacturer, PartNumber, Capacity, Speed, ConfiguredClockSpeed, SMBIOSMemoryType, MemoryType, FormFactor FROM Win32_PhysicalMemory",
        ) {
            Ok(rows) => {
                for row in rows {
                    let capacity_bytes = var_u64(&row, "Capacity").unwrap_or(0);
                    if capacity_bytes == 0 {
                        continue;
                    }
                    let rated_mts = var_u32(&row, "Speed").unwrap_or(0);
                    let configured_mts = var_u32(&row, "ConfiguredClockSpeed")
                        .filter(|&v| v > 0)
                        .unwrap_or(rated_mts);
                    let smbios = var_u32(&row, "SMBIOSMemoryType").unwrap_or(0);
                    let legacy = var_u32(&row, "MemoryType").unwrap_or(0);
                    let mem_type = map_smbios_memory_type(if smbios != 0 { smbios } else { legacy });
                    let form_code = var_u32(&row, "FormFactor").unwrap_or(0);
                    let form_factor = match form_code {
                        8 => "DIMM".to_string(),
                        12 => "SODIMM".to_string(),
                        _ => "Module".to_string(),
                    };

                    info.dimms.push(DimmStick {
                        slot: var_str(&row, "DeviceLocator").unwrap_or_else(|| "DIMM".to_string()),
                        manufacturer: var_str(&row, "Manufacturer")
                            .unwrap_or_else(|| "Unknown".to_string()),
                        part_number: var_str(&row, "PartNumber").unwrap_or_default(),
                        capacity_bytes,
                        configured_mts,
                        rated_mts,
                        memory_type: mem_type,
                        form_factor,
                    });
                }
            }
            Err(e) => diagnostics.push(format!("Win32_PhysicalMemory query failed: {e}")),
        }
    }

    info.used_slots = info.dimms.len();
    if info.total_slots < info.used_slots as u32 {
        info.total_slots = info.used_slots as u32;
    }

    if let Some(first) = info.dimms.first() {
        info.speed_mts = if first.configured_mts > 0 {
            first.configured_mts
        } else {
            first.rated_mts
        };
        info.memory_type = first.memory_type.clone();
    } else {
        info.memory_type = "RAM".to_string();
    }

    info.channel_mode = match info.used_slots {
        0 => "Unknown".to_string(),
        1 => "Single Channel".to_string(),
        2 => "Dual Channel".to_string(),
        4 => "Quad / Dual-Rank".to_string(),
        n => format!("{n} Modules"),
    };

    info
}

pub fn map_smbios_memory_type(code: u32) -> String {
    match code {
        20 => "DDR".to_string(),
        21 => "DDR2".to_string(),
        24 => "DDR3".to_string(),
        26 => "DDR4".to_string(),
        29 => "LPDDR3".to_string(),
        30 => "LPDDR4".to_string(),
        34 => "DDR5".to_string(),
        35 => "LPDDR5".to_string(),
        _ => "RAM".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::map_smbios_memory_type;

    #[test]
    fn smbios_memory_type_mapping_tests() {
        assert_eq!(map_smbios_memory_type(21), "DDR2");
        assert_eq!(map_smbios_memory_type(24), "DDR3");
        assert_eq!(map_smbios_memory_type(26), "DDR4");
        assert_eq!(map_smbios_memory_type(29), "LPDDR3");
        assert_eq!(map_smbios_memory_type(30), "LPDDR4");
        assert_eq!(map_smbios_memory_type(34), "DDR5");
        assert_eq!(map_smbios_memory_type(35), "LPDDR5");
        assert_eq!(map_smbios_memory_type(0), "RAM");
    }
}
