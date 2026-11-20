use crate::engine::from_wide_null;
use crate::models::{DisplayInfo, GpuInfo};
use std::collections::HashSet;
use windows::core::PCWSTR;
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIAdapter1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE,
};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayDevicesW, EnumDisplaySettingsW, DEVMODEW, DISPLAY_DEVICEW, ENUM_CURRENT_SETTINGS,
};
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};
use winreg::RegKey;

const DISPLAY_CLASS_GUID: &str = "{4d36e968-e325-11ce-bfc1-08002be10318}";
const DISPLAY_DEVICE_ACTIVE: u32 = 0x0000_0001;
const DISPLAY_DEVICE_PRIMARY_DEVICE: u32 = 0x0000_0004;

struct DriverClassMeta {
    desc: String,
    version: String,
    date: String,
    matching_id: String,
}

pub fn detect_gpus_and_displays(diagnostics: &mut Vec<String>) -> (Vec<GpuInfo>, Vec<DisplayInfo>, bool) {
    let class_metas = read_display_class_registry();
    let pci_msi_entries = read_pci_gpu_msi_states();
    let mut gpus = detect_dxgi_gpus(&class_metas, &pci_msi_entries, diagnostics);
    if gpus.is_empty() {
        for meta in &class_metas {
            gpus.push(GpuInfo {
                name: meta.desc.clone(),
                vendor: infer_vendor_from_name(&meta.desc),
                driver_version: meta.version.clone(),
                driver_date: meta.date.clone(),
                is_integrated: looks_integrated(&meta.desc, 0),
                ..Default::default()
            });
        }
    }

    let displays = detect_displays();
    let hags_enabled = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(
            r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers",
            KEY_READ | KEY_WOW64_64KEY,
        )
        .and_then(|k| k.get_value::<u32, _>("HwSchMode"))
        .map(|v| v == 2)
        .unwrap_or(false);

    (gpus, displays, hags_enabled)
}

fn detect_dxgi_gpus(
    class_metas: &[DriverClassMeta],
    pci_msi_entries: &[(String, Option<bool>)],
    diagnostics: &mut Vec<String>,
) -> Vec<GpuInfo> {
    let mut out = Vec::new();
    let mut seen_luids = HashSet::new();

    unsafe {
        let factory: IDXGIFactory1 = match CreateDXGIFactory1() {
            Ok(f) => f,
            Err(e) => {
                diagnostics.push(format!("CreateDXGIFactory1 failed: {e}"));
                return out;
            }
        };

        let mut idx = 0u32;
        while let Ok(adapter) = factory.EnumAdapters1(idx) {
            idx += 1;
            let adapter1: IDXGIAdapter1 = adapter;
            let Ok(desc) = adapter1.GetDesc1() else {
                continue;
            };
            if (desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0 {
                continue;
            }
            let luid_key = (desc.AdapterLuid.LowPart, desc.AdapterLuid.HighPart);
            if !seen_luids.insert(luid_key) {
                continue;
            }

            let name = from_wide_null(&desc.Description);
            let vram_bytes = desc.DedicatedVideoMemory as u64;
            let shared_sys_mem_bytes = desc.SharedSystemMemory as u64;
            let vendor = match desc.VendorId {
                0x10DE => "NVIDIA".to_string(),
                0x1002 => "AMD".to_string(),
                0x8086 => "Intel".to_string(),
                0x1414 => "Microsoft".to_string(),
                other => format!("0x{other:04X}"),
            };

            let ven_dev_needle = format!("ven_{:04x}&dev_{:04x}", desc.VendorId, desc.DeviceId);
            let name_lower = name.to_lowercase();

            let matched_meta = class_metas.iter().find(|m| {
                m.matching_id.to_lowercase().contains(&ven_dev_needle)
                    || m.desc.eq_ignore_ascii_case(&name)
                    || (!name_lower.is_empty() && m.desc.to_lowercase().contains(&name_lower))
            });

            let matched_pci = pci_msi_entries
                .iter()
                .find(|(pnp, _)| pnp.to_lowercase().contains(&ven_dev_needle));

            let is_integrated = looks_integrated_vendor(desc.VendorId, &name, vram_bytes);

            out.push(GpuInfo {
                name,
                vendor,
                vendor_id: desc.VendorId,
                device_id: desc.DeviceId,
                vram_bytes,
                shared_sys_mem_bytes,
                driver_version: matched_meta.map(|m| m.version.clone()).unwrap_or_default(),
                driver_date: matched_meta.map(|m| m.date.clone()).unwrap_or_default(),
                is_integrated,
                pnp_device_id: matched_pci.map(|(p, _)| p.clone()),
                msi_supported: matched_pci.and_then(|(_, msi)| *msi),
            });
        }
    }

    // Put discrete GPUs first, then integrated
    out.sort_by_key(|g| g.is_integrated);
    out
}

fn looks_integrated_vendor(vendor_id: u32, name: &str, vram_bytes: u64) -> bool {
    if vram_bytes == 0 {
        return true;
    }
    let lower = name.to_lowercase();
    if lower.contains("microsoft basic") {
        return true;
    }
    if vendor_id == 0x8086 {
        // Intel UHD / Iris / HD Graphics use UMA (< 512 MB dedicated); Intel Arc dGPUs have 4+ GB
        return vram_bytes < 1024 * 1024 * 1024;
    }
    if vendor_id == 0x1002 {
        // AMD APU integrated GPUs report small dedicated carveout (512 MB - 2 GB) with "Radeon(TM) Graphics" or "Vega"
        return vram_bytes <= 512 * 1024 * 1024
            || lower.contains("radeon(tm) graphics")
            || lower.contains("vega");
    }
    false
}

fn looks_integrated(name: &str, vram_bytes: u64) -> bool {
    let lower = name.to_lowercase();
    vram_bytes == 0
        || lower.contains("uhd graphics")
        || lower.contains("hd graphics")
        || lower.contains("iris")
        || lower.contains("radeon(tm) graphics")
}

fn infer_vendor_from_name(name: &str) -> String {
    let l = name.to_lowercase();
    if l.contains("nvidia") || l.contains("geforce") || l.contains("rtx") || l.contains("gtx") {
        "NVIDIA".to_string()
    } else if l.contains("amd") || l.contains("radeon") {
        "AMD".to_string()
    } else if l.contains("intel") {
        "Intel".to_string()
    } else {
        "Unknown".to_string()
    }
}

fn read_display_class_registry() -> Vec<DriverClassMeta> {
    let mut list = Vec::new();
    let base_path = format!(r"SYSTEM\CurrentControlSet\Control\Class\{DISPLAY_CLASS_GUID}");
    let Ok(base) =
        RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(&base_path, KEY_READ | KEY_WOW64_64KEY)
    else {
        return list;
    };

    for sub_name in base.enum_keys().flatten() {
        // Subkeys like "0000", "0001", "0002", ..., "0010", etc.
        if sub_name.len() != 4 || !sub_name.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if let Ok(sub) = base.open_subkey_with_flags(&sub_name, KEY_READ | KEY_WOW64_64KEY) {
            let desc: String = sub.get_value("DriverDesc").unwrap_or_default();
            if desc.is_empty() {
                continue;
            }
            let version: String = sub.get_value("DriverVersion").unwrap_or_default();
            let date: String = sub.get_value("DriverDate").unwrap_or_default();
            let matching_id: String = sub.get_value("MatchingDeviceId").unwrap_or_default();
            list.push(DriverClassMeta {
                desc,
                version,
                date,
                matching_id,
            });
        }
    }
    list
}

/// Strictly inspects only PCI devices whose ClassGUID matches Display adapters (`{4d36e968-...}`).
pub fn read_pci_gpu_msi_states() -> Vec<(String, Option<bool>)> {
    let mut results = Vec::new();
    let Ok(pci_root) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(
        r"SYSTEM\CurrentControlSet\Enum\PCI",
        KEY_READ | KEY_WOW64_64KEY,
    ) else {
        return results;
    };

    for dev_id in pci_root.enum_keys().flatten() {
        let Ok(dev_key) = pci_root.open_subkey_with_flags(&dev_id, KEY_READ | KEY_WOW64_64KEY) else {
            continue;
        };
        for inst_id in dev_key.enum_keys().flatten() {
            let Ok(inst_key) = dev_key.open_subkey_with_flags(&inst_id, KEY_READ | KEY_WOW64_64KEY)
            else {
                continue;
            };
            let class_guid: String = inst_key.get_value("ClassGUID").unwrap_or_default();
            if !class_guid.eq_ignore_ascii_case(DISPLAY_CLASS_GUID) {
                continue;
            }
            let pnp = format!(r"PCI\{dev_id}\{inst_id}");
            let msi = inst_key
                .open_subkey_with_flags(
                    r"Device Parameters\Interrupt Management\MessageSignaledInterruptProperties",
                    KEY_READ | KEY_WOW64_64KEY,
                )
                .and_then(|k| k.get_value::<u32, _>("MSISupported"))
                .ok()
                .map(|v| v == 1);
            results.push((pnp, msi));
        }
    }
    results
}

fn detect_displays() -> Vec<DisplayInfo> {
    let mut list = Vec::new();
    unsafe {
        let mut idx = 0u32;
        loop {
            let mut dd = DISPLAY_DEVICEW::default();
            dd.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
            if !EnumDisplayDevicesW(None, idx, &mut dd, 0).as_bool() {
                break;
            }
            idx += 1;

            if (dd.StateFlags & DISPLAY_DEVICE_ACTIVE) == 0 {
                continue;
            }

            let device_name = from_wide_null(&dd.DeviceName);
            let is_primary = (dd.StateFlags & DISPLAY_DEVICE_PRIMARY_DEVICE) != 0;

            let mut mon = DISPLAY_DEVICEW::default();
            mon.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
            let monitor_name =
                if EnumDisplayDevicesW(PCWSTR(dd.DeviceName.as_ptr()), 0, &mut mon, 0).as_bool() {
                    let s = from_wide_null(&mon.DeviceString);
                    if s.is_empty() {
                        from_wide_null(&dd.DeviceString)
                    } else {
                        s
                    }
                } else {
                    from_wide_null(&dd.DeviceString)
                };

            let mut dm = DEVMODEW::default();
            dm.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
            if EnumDisplaySettingsW(PCWSTR(dd.DeviceName.as_ptr()), ENUM_CURRENT_SETTINGS, &mut dm)
                .as_bool()
            {
                list.push(DisplayInfo {
                    device_name,
                    monitor_name,
                    width: dm.dmPelsWidth,
                    height: dm.dmPelsHeight,
                    refresh_hz: dm.dmDisplayFrequency,
                    bits_per_pixel: dm.dmBitsPerPel,
                    is_primary,
                });
            }
        }
    }
    list.sort_by_key(|d| !d.is_primary);
    list
}
