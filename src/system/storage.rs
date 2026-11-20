use super::{var_str, var_u32, var_u64};
use crate::engine::{from_wide_null, to_wide_null};
use crate::models::{PhysicalDiskInfo, VolumeInfo};
use std::collections::HashMap;
use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDriveStringsW, GetVolumeInformationW,
};
use wmi::{Variant, WMIConnection};

const DRIVE_FIXED: u32 = 3;

pub fn detect_storage(
    wmi_storage: Option<&WMIConnection>,
    wmi_cimv2: Option<&WMIConnection>,
    diagnostics: &mut Vec<String>,
) -> (Vec<PhysicalDiskInfo>, Vec<VolumeInfo>) {
    let mut disks = Vec::new();

    if let Some(con) = wmi_storage {
        match con.raw_query::<HashMap<String, Variant>>(
            "SELECT FriendlyName, MediaType, BusType, HealthStatus, Size FROM MSFT_PhysicalDisk",
        ) {
            Ok(rows) => {
                for row in rows {
                    let friendly_name =
                        var_str(&row, "FriendlyName").unwrap_or_else(|| "Physical Disk".to_string());
                    let media_code = var_u32(&row, "MediaType").unwrap_or(0);
                    let bus_code = var_u32(&row, "BusType").unwrap_or(0);
                    let health_code = var_u32(&row, "HealthStatus").unwrap_or(0);
                    let size_bytes = var_u64(&row, "Size").unwrap_or(0);

                    let bus_type = match bus_code {
                        17 => "NVMe",
                        11 => "SATA",
                        7 => "USB",
                        8 => "RAID",
                        10 => "SAS",
                        1 => "SCSI",
                        9 => "iSCSI",
                        14 => "SD",
                        15 => "MMC",
                        _ => "Other",
                    }
                    .to_string();

                    let media_type = match (media_code, bus_code) {
                        (4, 17) => "NVMe SSD",
                        (4, 11) => "SATA SSD",
                        (4, _) => "SSD",
                        (3, _) => "HDD",
                        (5, _) => "SCM",
                        _ => "Storage",
                    }
                    .to_string();

                    let health_status = match health_code {
                        0 => "Healthy",
                        1 => "Warning",
                        2 => "Unhealthy",
                        _ => "Unknown",
                    }
                    .to_string();

                    disks.push(PhysicalDiskInfo {
                        friendly_name,
                        media_type,
                        bus_type,
                        health_status,
                        size_bytes,
                    });
                }
            }
            Err(e) => diagnostics.push(format!("MSFT_PhysicalDisk query failed: {e}")),
        }
    }

    // Fallback to Win32_DiskDrive ONLY if MSFT_PhysicalDisk yielded nothing (treat MediaType safely as String!)
    if disks.is_empty() {
        if let Some(con) = wmi_cimv2 {
            match con.raw_query::<HashMap<String, Variant>>(
                "SELECT Model, InterfaceType, MediaType, Status, Size FROM Win32_DiskDrive",
            ) {
                Ok(rows) => {
                    for row in rows {
                        let friendly_name =
                            var_str(&row, "Model").unwrap_or_else(|| "Physical Disk".to_string());
                        let iface = var_str(&row, "InterfaceType").unwrap_or_else(|| "Disk".to_string());
                        let media_raw = var_str(&row, "MediaType").unwrap_or_default();
                        let status = var_str(&row, "Status").unwrap_or_else(|| "OK".to_string());
                        let size_bytes = var_u64(&row, "Size").unwrap_or(0);

                        let name_upper = friendly_name.to_uppercase();
                        let media_type = if name_upper.contains("NVME") {
                            "NVMe SSD"
                        } else if name_upper.contains("SSD") {
                            "SSD"
                        } else if !media_raw.is_empty() {
                            &media_raw
                        } else {
                            "Fixed Disk"
                        }
                        .to_string();

                        disks.push(PhysicalDiskInfo {
                            friendly_name,
                            media_type,
                            bus_type: iface,
                            health_status: status,
                            size_bytes,
                        });
                    }
                }
                Err(e) => diagnostics.push(format!("Win32_DiskDrive fallback failed: {e}")),
            }
        }
    }

    let volumes = detect_fixed_volumes();
    (disks, volumes)
}

fn detect_fixed_volumes() -> Vec<VolumeInfo> {
    let mut volumes = Vec::new();
    unsafe {
        let needed = GetLogicalDriveStringsW(None);
        if needed == 0 {
            return volumes;
        }
        let mut buf = vec![0u16; needed as usize + 1];
        let written = GetLogicalDriveStringsW(Some(&mut buf));
        if written == 0 {
            return volumes;
        }

        let mut start = 0usize;
        while start < written as usize {
            let Some(rel_end) = buf[start..].iter().position(|&c| c == 0) else {
                break;
            };
            if rel_end == 0 {
                break;
            }
            let slice = &buf[start..start + rel_end];
            start += rel_end + 1;

            let mount = String::from_utf16_lossy(slice);
            let wide = to_wide_null(&mount);
            let drive_type = GetDriveTypeW(PCWSTR(wide.as_ptr()));
            if drive_type != DRIVE_FIXED {
                continue;
            }

            let mut free_avail = 0u64;
            let mut total_bytes = 0u64;
            let mut total_free = 0u64;
            if GetDiskFreeSpaceExW(
                PCWSTR(wide.as_ptr()),
                Some(&mut free_avail),
                Some(&mut total_bytes),
                Some(&mut total_free),
            )
            .is_err()
            {
                continue;
            }

            let mut label_buf = [0u16; 256];
            let mut fs_buf = [0u16; 64];
            let _ = GetVolumeInformationW(
                PCWSTR(wide.as_ptr()),
                Some(&mut label_buf),
                None,
                None,
                None,
                Some(&mut fs_buf),
            );

            let label = {
                let s = from_wide_null(&label_buf);
                if s.is_empty() {
                    "Local Disk".to_string()
                } else {
                    s
                }
            };
            let filesystem = from_wide_null(&fs_buf);
            let used_bytes = total_bytes.saturating_sub(free_avail);
            let used_pct = if total_bytes > 0 {
                (used_bytes as f64 / total_bytes as f64 * 100.0) as f32
            } else {
                0.0
            };

            volumes.push(VolumeInfo {
                mount,
                label,
                filesystem,
                total_bytes,
                free_bytes: free_avail,
                used_bytes,
                used_pct,
            });
        }
    }
    volumes
}
