use super::{var_str, var_u32};
use crate::models::CpuInfo;
use std::collections::{BTreeSet, HashMap};
use windows::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;
use windows::Win32::System::SystemInformation::{
    GetLogicalProcessorInformationEx, RelationAll, RelationCache, RelationProcessorCore,
    SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
};
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};
use winreg::RegKey;
use wmi::{Variant, WMIConnection};

pub fn detect_cpu(wmi_con: Option<&WMIConnection>, diagnostics: &mut Vec<String>) -> CpuInfo {
    let mut info = CpuInfo::default();
    info.architecture = std::env::consts::ARCH.to_string();

    if let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(
        r"HARDWARE\DESCRIPTION\System\CentralProcessor\0",
        KEY_READ | KEY_WOW64_64KEY,
    ) {
        if let Ok(name) = key.get_value::<String, _>("ProcessorNameString") {
            info.brand = name.trim().to_string();
        }
        if let Ok(vendor) = key.get_value::<String, _>("VendorIdentifier") {
            info.vendor = vendor.trim().to_string();
        }
        if let Ok(mhz) = key.get_value::<u32, _>("~MHz") {
            info.base_mhz = mhz;
        }
    }

    if let Some(con) = wmi_con {
        match con.raw_query::<HashMap<String, Variant>>(
            "SELECT Name, SocketDesignation, MaxClockSpeed FROM Win32_Processor",
        ) {
            Ok(rows) => {
                if let Some(first) = rows.first() {
                    if info.brand.is_empty() {
                        if let Some(n) = var_str(first, "Name") {
                            info.brand = n;
                        }
                    }
                    info.socket = var_str(first, "SocketDesignation").unwrap_or_default();
                    info.max_mhz = var_u32(first, "MaxClockSpeed").unwrap_or(info.base_mhz);
                }
            }
            Err(e) => diagnostics.push(format!("Win32_Processor query failed: {e}")),
        }
    }

    populate_processor_topology(&mut info, diagnostics);

    if info.brand.is_empty() {
        info.brand = "x86_64 Processor".to_string();
    }
    if info.max_mhz == 0 {
        info.max_mhz = info.base_mhz;
    }

    info
}

fn populate_processor_topology(info: &mut CpuInfo, diagnostics: &mut Vec<String>) {
    unsafe {
        let mut len: u32 = 0;
        let _ = GetLogicalProcessorInformationEx(RelationAll, None, &mut len);
        if len == 0 {
            let err = windows::Win32::Foundation::GetLastError();
            if err != ERROR_INSUFFICIENT_BUFFER {
                diagnostics.push(format!(
                    "GetLogicalProcessorInformationEx sizing failed: {:?}",
                    err
                ));
                return;
            }
        }

        let mut buf = vec![0u8; len as usize];
        let res = GetLogicalProcessorInformationEx(
            RelationAll,
            Some(buf.as_mut_ptr() as *mut SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX),
            &mut len,
        );
        if res.is_err() {
            diagnostics.push("GetLogicalProcessorInformationEx query failed".to_string());
            return;
        }

        let mut cores: Vec<(u8, u64)> = Vec::new();
        let mut eff_classes: BTreeSet<u8> = BTreeSet::new();
        let mut logical_threads: usize = 0;
        let mut l1_kb: u32 = 0;
        let mut l2_kb: u32 = 0;
        let mut l3_entries: Vec<(u32, u64)> = Vec::new();

        let mut offset: usize = 0;
        while offset + std::mem::size_of::<u32>() * 2 <= len as usize {
            let ptr = buf.as_ptr().add(offset) as *const SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX;
            let entry = &*ptr;
            if entry.Size == 0 {
                break;
            }

            if entry.Relationship == RelationProcessorCore {
                let proc_rel = entry.Anonymous.Processor;
                let eff = proc_rel.EfficiencyClass;
                eff_classes.insert(eff);

                let group_count = proc_rel.GroupCount as usize;
                let masks_ptr = proc_rel.GroupMask.as_ptr();
                let mut g0_mask = 0u64;
                for g in 0..group_count {
                    let ga = *masks_ptr.add(g);
                    logical_threads += ga.Mask.count_ones() as usize;
                    if ga.Group == 0 {
                        g0_mask |= ga.Mask as u64;
                    }
                }
                cores.push((eff, g0_mask));
            } else if entry.Relationship == RelationCache {
                let cache = entry.Anonymous.Cache;
                let kb = cache.CacheSize / 1024;
                match cache.Level {
                    1 => l1_kb = l1_kb.saturating_add(kb),
                    2 => l2_kb = l2_kb.saturating_add(kb),
                    3 => {
                        let mask = cache.Anonymous.GroupMask.Mask as u64;
                        l3_entries.push((kb, mask));
                    }
                    _ => {}
                }
            }

            offset += entry.Size as usize;
        }

        info.physical_cores = cores.len();
        info.logical_threads = logical_threads;
        info.smt_enabled = logical_threads > cores.len();
        info.l1_kb = l1_kb;
        info.l2_kb = l2_kb;
        info.l3_kb = l3_entries.iter().map(|(kb, _)| *kb).sum();

        let brand_upper = info.brand.to_uppercase();
        let is_amd = brand_upper.contains("AMD") || info.vendor.contains("AuthenticAMD");
        let is_x3d_brand = brand_upper.contains("X3D");

        if eff_classes.len() > 1 {
            let max_eff = *eff_classes.iter().next_back().unwrap_or(&0);
            info.is_hybrid = true;
            for (eff, g0_mask) in &cores {
                if *eff == max_eff {
                    info.p_core_count += 1;
                    info.p_core_mask |= *g0_mask;
                } else {
                    info.e_core_count += 1;
                }
            }
            info.topology_label = format!(
                "Hybrid ({} P-Cores / {} E-Cores)",
                info.p_core_count, info.e_core_count
            );
        } else {
            info.p_core_count = info.physical_cores;
            info.e_core_count = 0;
            info.p_core_mask = compute_group0_all_mask(logical_threads);

            if is_amd && l3_entries.len() >= 2 {
                let max_l3 = l3_entries.iter().map(|(kb, _)| *kb).max().unwrap_or(0);
                let min_l3 = l3_entries.iter().map(|(kb, _)| *kb).min().unwrap_or(0);
                if max_l3 > min_l3 || is_x3d_brand {
                    info.is_x3d = true;
                    let mut mask = 0u64;
                    for (kb, m) in &l3_entries {
                        if *kb == max_l3 {
                            mask |= *m;
                        }
                    }
                    info.x3d_ccd0_mask = mask;
                    info.topology_label = if max_l3 > min_l3 {
                        "AMD Ryzen Dual-CCD 3D V-Cache".to_string()
                    } else {
                        "AMD Ryzen 3D V-Cache".to_string()
                    };
                } else {
                    info.topology_label = format!("AMD Dual-CCD ({} Cores)", info.physical_cores);
                }
            } else if is_x3d_brand {
                info.is_x3d = true;
                info.x3d_ccd0_mask = info.p_core_mask;
                info.topology_label = "AMD Ryzen 3D V-Cache".to_string();
            } else {
                info.topology_label = format!(
                    "Symmetric ({} Cores / {} Threads)",
                    info.physical_cores, info.logical_threads
                );
            }
        }
    }
}

pub fn compute_group0_all_mask(logical_threads: usize) -> u64 {
    if logical_threads == 0 {
        return 1;
    }
    if logical_threads >= 64 {
        u64::MAX
    } else {
        (1u64 << logical_threads) - 1
    }
}
