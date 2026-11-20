use super::{var_bool, var_str};
use crate::engine::{from_wide_null, run_command_timeout, system_drive_root};
use crate::models::{PlatformInfo, PowerInfo, SecurityInfo};
use std::collections::HashMap;
use std::path::Path;
use windows::core::{s, w};
use windows::Win32::Foundation::{LocalFree, HLOCAL};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Power::{
    GetSystemPowerStatus, PowerGetActiveScheme, PowerReadFriendlyName, SYSTEM_POWER_STATUS,
};
use windows::Win32::System::SystemInformation::GetTickCount64;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};
use winreg::RegKey;
use wmi::{Variant, WMIConnection};

pub fn detect_platform(
    wmi_con: Option<&WMIConnection>,
    diagnostics: &mut Vec<String>,
) -> PlatformInfo {
    let mut info = PlatformInfo {
        architecture: std::env::consts::ARCH.to_string(),
        computer_name: std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Windows-PC".to_string()),
        user_name: std::env::var("USERNAME").unwrap_or_else(|_| "User".to_string()),
        uptime_secs: unsafe { GetTickCount64() / 1000 },
        ..Default::default()
    };

    if let Ok(nt_key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        KEY_READ | KEY_WOW64_64KEY,
    ) {
        let edition: String = nt_key.get_value("EditionID").unwrap_or_default();
        let display_ver: String = nt_key.get_value("DisplayVersion").unwrap_or_default();
        let build_str: String = nt_key.get_value("CurrentBuildNumber").unwrap_or_default();
        let product_name: String = nt_key.get_value("ProductName").unwrap_or_default();
        let ubr: u32 = nt_key.get_value("UBR").unwrap_or(0);

        let build_number = build_str.trim().parse::<u32>().unwrap_or(0);
        let is_win11 = build_number >= 22000;
        let base_os = if is_win11 {
            "Windows 11"
        } else if build_number >= 10240 {
            "Windows 10"
        } else if !product_name.is_empty() {
            &product_name
        } else {
            "Windows"
        };

        info.edition = edition.clone();
        info.display_version = display_ver.clone();
        info.build_number = build_number;
        info.ubr = ubr;
        info.is_win11 = is_win11;
        info.os_name = if edition.is_empty() {
            base_os.to_string()
        } else {
            format!("{base_os} {edition}")
        };
    }

    let mut reboot_reasons = Vec::new();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if hklm
        .open_subkey_with_flags(
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Component Based Servicing\RebootPending",
            KEY_READ | KEY_WOW64_64KEY,
        )
        .is_ok()
    {
        reboot_reasons.push("Component Based Servicing".to_string());
    }
    if hklm
        .open_subkey_with_flags(
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\WindowsUpdate\Auto Update\RebootRequired",
            KEY_READ | KEY_WOW64_64KEY,
        )
        .is_ok()
    {
        reboot_reasons.push("Windows Update".to_string());
    }
    if let Ok(sm) = hklm.open_subkey_with_flags(
        r"SYSTEM\CurrentControlSet\Control\Session Manager",
        KEY_READ | KEY_WOW64_64KEY,
    ) {
        if let Ok(pending) = sm.get_value::<Vec<String>, _>("PendingFileRenameOperations") {
            if !pending.is_empty() {
                reboot_reasons.push("PendingFileRenameOperations".to_string());
            }
        }
    }
    info.pending_reboot = !reboot_reasons.is_empty();
    info.pending_reboot_reasons = reboot_reasons;

    if let Some(con) = wmi_con {
        if let Ok(rows) = con.raw_query::<HashMap<String, Variant>>(
            "SELECT Manufacturer, Product FROM Win32_BaseBoard",
        ) {
            if let Some(r) = rows.first() {
                let mfg = var_str(r, "Manufacturer").unwrap_or_default();
                let prod = var_str(r, "Product").unwrap_or_default();
                info.motherboard = format!("{} {}", mfg, prod).trim().to_string();
            }
        } else {
            diagnostics.push("Win32_BaseBoard query failed".to_string());
        }

        if let Ok(rows) = con.raw_query::<HashMap<String, Variant>>(
            "SELECT Manufacturer, SMBIOSBIOSVersion, ReleaseDate FROM Win32_BIOS",
        ) {
            if let Some(r) = rows.first() {
                info.bios_vendor = var_str(r, "Manufacturer").unwrap_or_default();
                info.bios_version = var_str(r, "SMBIOSBIOSVersion").unwrap_or_default();
                if let Some(raw_date) = var_str(r, "ReleaseDate") {
                    info.bios_date = format_dmtf_date_short(&raw_date);
                }
            }
        }

        if let Ok(rows) = con.raw_query::<HashMap<String, Variant>>(
            "SELECT ChassisTypes FROM Win32_SystemEnclosure",
        ) {
            if let Some(r) = rows.first() {
                if let Some(Variant::Array(arr)) = r.get("ChassisTypes") {
                    for item in arr {
                        let code = match item {
                            Variant::I4(v) => Some(*v as u32),
                            Variant::UI4(v) => Some(*v),
                            Variant::UI2(v) => Some(*v as u32),
                            Variant::I2(v) => Some(*v as u32),
                            _ => None,
                        };
                        if let Some(c) = code {
                            let (label, laptop) = map_chassis_type(c);
                            info.chassis_type = label.to_string();
                            info.is_laptop = laptop;
                            break;
                        }
                    }
                }
            }
        }
    }

    unsafe {
        let mut sps = SYSTEM_POWER_STATUS::default();
        if GetSystemPowerStatus(&mut sps).is_ok() {
            let no_battery = (sps.BatteryFlag & 128) != 0 || sps.BatteryFlag == 255;
            if !no_battery {
                info.battery_present = true;
                info.is_laptop = true;
                if sps.BatteryLifePercent <= 100 {
                    info.battery_pct = Some(sps.BatteryLifePercent);
                }
                if sps.ACLineStatus == 0 || sps.ACLineStatus == 1 {
                    info.ac_online = Some(sps.ACLineStatus == 1);
                }
            }
        }
    }

    if info.chassis_type.is_empty() {
        info.chassis_type = if info.is_laptop {
            "Laptop / Notebook".to_string()
        } else {
            "Desktop".to_string()
        };
    }

    info
}

pub fn detect_security(
    wmi_tpm: Option<&WMIConnection>,
    hags_enabled: bool,
    diagnostics: &mut Vec<String>,
) -> SecurityInfo {
    let mut sec = SecurityInfo {
        hags_enabled,
        hypervisor_launch_type: "Unknown".to_string(),
        ..Default::default()
    };

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    sec.secure_boot = hklm
        .open_subkey_with_flags(
            r"SYSTEM\CurrentControlSet\Control\SecureBoot\State",
            KEY_READ | KEY_WOW64_64KEY,
        )
        .and_then(|k| k.get_value::<u32, _>("UEFISecureBootEnabled"))
        .ok()
        .map(|v| v == 1);

    sec.hvci_enabled = hklm
        .open_subkey_with_flags(
            r"SYSTEM\CurrentControlSet\Control\DeviceGuard\Scenarios\HypervisorEnforcedCodeIntegrity",
            KEY_READ | KEY_WOW64_64KEY,
        )
        .and_then(|k| k.get_value::<u32, _>("Enabled"))
        .map(|v| v == 1)
        .unwrap_or(false);

    if let Some(con) = wmi_tpm {
        match con.raw_query::<HashMap<String, Variant>>(
            "SELECT SpecVersion, IsEnabled_InitialValue, IsActivated_InitialValue FROM Win32_Tpm",
        ) {
            Ok(rows) => {
                if let Some(r) = rows.first() {
                    sec.tpm_present = true;
                    sec.tpm_enabled = var_bool(r, "IsEnabled_InitialValue").unwrap_or(false)
                        && var_bool(r, "IsActivated_InitialValue").unwrap_or(false);
                    sec.tpm_spec_version = var_str(r, "SpecVersion").unwrap_or_default();
                }
            }
            Err(e) => diagnostics.push(format!("Win32_Tpm query skipped/denied: {e}")),
        }
    }

    let hypervisor = parse_hypervisor_launch_type();
    sec.vbs_active = sec.hvci_enabled || hypervisor.eq_ignore_ascii_case("Auto");
    sec.hypervisor_launch_type = hypervisor;

    sec.game_mode_enabled = hkcu
        .open_subkey_with_flags(r"Software\Microsoft\GameBar", KEY_READ | KEY_WOW64_64KEY)
        .and_then(|k| k.get_value::<u32, _>("AutoGameModeEnabled"))
        .map(|v| v != 0)
        .unwrap_or(true);

    let app_cap = hkcu
        .open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\GameDVR",
            KEY_READ | KEY_WOW64_64KEY,
        )
        .and_then(|k| k.get_value::<u32, _>("AppCaptureEnabled"))
        .unwrap_or(1);
    sec.game_dvr_enabled = app_cap != 0;

    sec
}

pub fn parse_hypervisor_launch_type() -> String {
    if let Ok(out) = run_command_timeout("bcdedit.exe", &["/enum", "{current}"], 2500) {
        if out.exit_code == 0 {
            for line in out.stdout.lines() {
                let lower = line.to_ascii_lowercase();
                if lower.trim_start().starts_with("hypervisorlaunchtype") {
                    if let Some(val) = line.split_whitespace().last() {
                        return val.to_string();
                    }
                }
            }
            return "Not Set (Off)".to_string();
        }
    }
    "Unknown (Admin required)".to_string()
}

pub fn detect_power(diagnostics: &mut Vec<String>) -> PowerInfo {
    let mut info = PowerInfo {
        system_drive: system_drive_root(),
        ..Default::default()
    };

    if let Some((guid, name)) = get_active_power_scheme() {
        info.active_scheme_guid = guid;
        info.active_scheme_name = name;
    } else {
        diagnostics.push("PowerGetActiveScheme unavailable".to_string());
    }

    let (min_ms, max_ms, cur_ms) = query_timer_resolution_ms();
    info.timer_res_min_ms = min_ms;
    info.timer_res_max_ms = max_ms;
    info.timer_res_current_ms = cur_ms;

    let hiber_path = format!("{}hiberfil.sys", info.system_drive);
    if let Ok(meta) = std::fs::metadata(Path::new(&hiber_path)) {
        info.hibernation_enabled = true;
        info.hiberfil_bytes = meta.len();
    }

    info
}

pub fn get_active_power_scheme() -> Option<(String, String)> {
    unsafe {
        let mut guid_ptr: *mut windows::core::GUID = std::ptr::null_mut();
        let err = PowerGetActiveScheme(None, &mut guid_ptr);
        if err.0 != 0 || guid_ptr.is_null() {
            return None;
        }
        let guid = *guid_ptr;
        let _ = LocalFree(HLOCAL(guid_ptr as _));

        let guid_str = format_guid(&guid);
        let name = read_power_scheme_name(&guid).unwrap_or_else(|| guid_str.clone());
        Some((guid_str, name))
    }
}

pub fn read_power_scheme_name(guid: &windows::core::GUID) -> Option<String> {
    unsafe {
        let mut size = 0u32;
        let _ = PowerReadFriendlyName(None, Some(guid), None, None, None, &mut size);
        if size == 0 {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        let err = PowerReadFriendlyName(
            None,
            Some(guid),
            None,
            None,
            Some(buf.as_mut_ptr()),
            &mut size,
        );
        if err.0 != 0 {
            return None;
        }
        let u16_slice: Vec<u16> = buf[..size as usize]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let s = from_wide_null(&u16_slice);
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

pub fn format_guid(g: &windows::core::GUID) -> String {
    format!(
        "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        g.data1,
        g.data2,
        g.data3,
        g.data4[0],
        g.data4[1],
        g.data4[2],
        g.data4[3],
        g.data4[4],
        g.data4[5],
        g.data4[6],
        g.data4[7]
    )
}

pub fn parse_guid(s: &str) -> Option<windows::core::GUID> {
    let clean = s.trim().trim_matches(|c| c == '{' || c == '}');
    let parts: Vec<&str> = clean.split('-').collect();
    if parts.len() != 5 {
        return None;
    }
    let data1 = u32::from_str_radix(parts[0], 16).ok()?;
    let data2 = u16::from_str_radix(parts[1], 16).ok()?;
    let data3 = u16::from_str_radix(parts[2], 16).ok()?;
    if parts[3].len() != 4 || parts[4].len() != 12 {
        return None;
    }
    let mut data4 = [0u8; 8];
    data4[0] = u8::from_str_radix(&parts[3][0..2], 16).ok()?;
    data4[1] = u8::from_str_radix(&parts[3][2..4], 16).ok()?;
    for i in 0..6 {
        data4[2 + i] = u8::from_str_radix(&parts[4][i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(windows::core::GUID::from_values(data1, data2, data3, data4))
}

pub fn query_timer_resolution_ms() -> (f64, f64, f64) {
    type NtQueryFn = unsafe extern "system" fn(*mut u32, *mut u32, *mut u32) -> i32;
    unsafe {
        if let Ok(hmod) = GetModuleHandleW(w!("ntdll.dll")) {
            if let Some(proc_addr) = GetProcAddress(hmod, s!("NtQueryTimerResolution")) {
                let f: NtQueryFn = std::mem::transmute(proc_addr);
                let (mut min_r, mut max_r, mut cur_r) = (0u32, 0u32, 0u32);
                if f(&mut min_r, &mut max_r, &mut cur_r) == 0 && cur_r > 0 {
                    return (
                        min_r as f64 / 10_000.0,
                        max_r as f64 / 10_000.0,
                        cur_r as f64 / 10_000.0,
                    );
                }
            }
        }
    }
    (15.625, 0.5, 15.625)
}

fn map_chassis_type(code: u32) -> (&'static str, bool) {
    match code {
        8 | 9 | 10 | 11 | 12 | 14 | 31 | 32 => ("Laptop / Notebook", true),
        30 => ("Tablet", true),
        3 | 4 | 6 | 7 | 13 | 15 | 16 => ("Desktop Workstation", false),
        35 => ("Mini PC", false),
        _ => ("PC System", false),
    }
}

pub fn format_dmtf_date_short(dmtf: &str) -> String {
    if dmtf.len() >= 8 && dmtf.as_bytes()[..8].iter().all(|b| b.is_ascii_digit()) {
        format!("{}-{}-{}", &dmtf[0..4], &dmtf[4..6], &dmtf[6..8])
    } else {
        dmtf.to_string()
    }
}

pub fn format_dmtf_datetime(dmtf: &str) -> String {
    if dmtf.len() >= 14 && dmtf.as_bytes()[..14].iter().all(|b| b.is_ascii_digit()) {
        format!(
            "{}-{}-{} {}:{}:{}",
            &dmtf[0..4],
            &dmtf[4..6],
            &dmtf[6..8],
            &dmtf[8..10],
            &dmtf[10..12],
            &dmtf[12..14]
        )
    } else {
        dmtf.to_string()
    }
}
