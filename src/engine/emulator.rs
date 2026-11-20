use crate::engine::cleaner::{collect_matching_files, purge_specs_safe};
use crate::engine::run_command_timeout;
use crate::engine::safety::{
    atomic_write_utf8_no_bom, read_reg_dword, read_reg_string, reg_delete_value_journaled,
    reg_set_journaled,
};
use crate::engine::tweaks::{
    apply_mmcss_network_throttle, apply_mouse_acceleration, apply_nagle_on_primary,
    check_mmcss_optimized, check_nagle_disabled_on_primary,
};
use crate::models::{
    CleanerTargetSpec, EmulatorTopology, RegHive, TransactionJournal, TypedRegValue,
};
use crate::system::ecosystem::is_process_running_ci;
use crate::system::platform::parse_hypervisor_launch_type;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};
use winreg::RegKey;

const GPU_PREF_SUBKEY: &str = r"Software\Microsoft\DirectX\UserGpuPreferences";
const APP_COMPAT_LAYERS: &str =
    r"Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers";
const IFEO_BASE_SUBKEY: &str =
    r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options";

/// Strictly the VM + render engine executables (excludes BstkSVC.exe / HD-Adb.exe so service threads never starve render threads).
const EMULATOR_RENDER_IFEO_EXES: &[&str] = &[
    "HD-Player.exe",
    "BstkVMMgr.exe",
    "dnplayer.exe",
    "LdVBoxHeadless.exe",
    "NoxVMHandle.exe",
    "MuMuVMMHeadless.exe",
];

/// Resolves `(Version, InstallDir, UserDefinedDir, LogDir)` from `HKLM\SOFTWARE\<reg_name>` with standard ProgramData/ProgramFiles fallbacks.
fn query_bluestacks_variant_paths(
    product_folder: &str,
) -> (Option<String>, PathBuf, PathBuf, PathBuf) {
    let pd = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".to_string());
    let pf = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string());

    let mut version = None;
    let mut install_dir = Path::new(&pf).join(product_folder);
    let mut user_dir = Path::new(&pd).join(product_folder);
    let mut log_dir = user_dir.join("Logs");

    if let Ok(k) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(
        format!(r"SOFTWARE\{product_folder}"),
        KEY_READ | KEY_WOW64_64KEY,
    ) {
        if let Ok(v) = k.get_value::<String, _>("Version") {
            if !v.trim().is_empty() {
                version = Some(v.trim().to_string());
            }
        }
        if let Ok(idir) = k.get_value::<String, _>("InstallDir") {
            let p = PathBuf::from(idir.trim().trim_end_matches('\\'));
            if p.exists() {
                install_dir = p;
            }
        }
        if let Ok(udir) = k.get_value::<String, _>("UserDefinedDir") {
            let p = PathBuf::from(udir.trim().trim_end_matches('\\'));
            if p.exists() {
                user_dir = p;
            }
        }
        if let Ok(ldir) = k.get_value::<String, _>("LogDir") {
            let p = PathBuf::from(ldir.trim().trim_end_matches('\\'));
            if p.exists() {
                log_dir = p;
            }
        }
    }

    (version, install_dir, user_dir.join("bluestacks.conf"), log_dir)
}

pub fn detect_topology() -> EmulatorTopology {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    let (bs_ver, _bs_install, bs_conf_path, _bs_logs) =
        query_bluestacks_variant_paths("BlueStacks_nxt");
    let bs_installed = bs_conf_path.is_file()
        || bs_ver.is_some()
        || !existing_bs_render_exes("BlueStacks_nxt").is_empty();

    let (msi_ver, _msi_install, msi_conf_path, _msi_logs) =
        query_bluestacks_variant_paths("BlueStacks_msi5");
    let msi_installed = msi_conf_path.is_file()
        || msi_ver.is_some()
        || !existing_bs_render_exes("BlueStacks_msi5").is_empty();

    let ld_exe = detect_ldplayer_exe();
    let ld_installed = ld_exe.is_some();
    let ldplayer_configs = find_ldplayer_configs(ld_exe.as_deref());

    let mut instances_found = Vec::new();
    let mut fps_240_unlocked = false;
    let mut rog_profile_active = false;
    let mut astc_and_debloat_active = false;

    for conf in [&bs_conf_path, &msi_conf_path] {
        if let Ok(raw) = std::fs::read_to_string(conf) {
            for inst in parse_bluestacks_instances(&raw) {
                if !instances_found.contains(&inst) {
                    instances_found.push(inst);
                }
            }
            let summary = audit_bluestacks_conf(&raw);
            fps_240_unlocked |= summary.0;
            rog_profile_active |= summary.1;
            astc_and_debloat_active |= summary.2;
        }
    }

    let ldplayer_optimized = !ldplayer_configs.is_empty()
        && ldplayer_configs.iter().all(|p| {
            std::fs::read_to_string(p)
                .map(|s| {
                    s.contains("ASUS_I005DA")
                        && s.contains("\"basicSettings.highFp\": true")
                        && (s.contains("240") || s.contains("120"))
                })
                .unwrap_or(false)
        });

    let ifeo_priority_locked = read_reg_dword(
        RegHive::Hklm,
        &format!(r"{IFEO_BASE_SUBKEY}\HD-Player.exe\PerfOptions"),
        "CpuPriorityClass",
    ) == Some(3);

    let candidate_exes = collect_existing_emulator_render_exes();
    let gpu_locked = if candidate_exes.is_empty() {
        false
    } else if let Ok(k) = hkcu.open_subkey_with_flags(GPU_PREF_SUBKEY, KEY_READ) {
        candidate_exes.iter().all(|exe| {
            k.get_value::<String, _>(exe)
                .map(|v| v.contains("GpuPreference=2"))
                .unwrap_or(false)
        })
    } else {
        false
    };

    let raw_mouse_active = read_reg_string(RegHive::Hkcu, r"Control Panel\Mouse", "MouseSpeed")
        .as_deref()
        == Some("0")
        && read_reg_string(RegHive::Hkcu, r"Control Panel\Mouse", "MouseThreshold1").as_deref()
            == Some("0");

    let low_latency_net_active =
        check_nagle_disabled_on_primary() && check_mmcss_optimized();

    let hypervisor = parse_hypervisor_launch_type();
    let hvci = read_reg_dword(
        RegHive::Hklm,
        r"SYSTEM\CurrentControlSet\Control\DeviceGuard\Scenarios\HypervisorEnforcedCodeIntegrity",
        "Enabled",
    ) == Some(1);
    let hypervisor_off = hypervisor.eq_ignore_ascii_case("Off") && !hvci;
    let hypervisor_status = format!(
        "Hypervisor: {} | HVCI: {}",
        hypervisor,
        if hvci { "ON" } else { "OFF" }
    );

    let log_specs = emulator_log_specs();
    let cache_bytes = collect_matching_files(&log_specs)
        .iter()
        .map(|(_, s)| *s)
        .sum();

    EmulatorTopology {
        bluestacks_installed: bs_installed,
        bluestacks_version: bs_ver,
        bluestacks_conf: bs_conf_path
            .is_file()
            .then(|| bs_conf_path.to_string_lossy().to_string()),
        msi_installed,
        msi_version: msi_ver,
        msi_conf: msi_conf_path
            .is_file()
            .then(|| msi_conf_path.to_string_lossy().to_string()),
        ldplayer_installed: ld_installed,
        ldplayer_exe: ld_exe,
        ldplayer_configs,
        instances_found,
        fps_240_unlocked,
        rog_profile_active,
        astc_and_debloat_active,
        ldplayer_optimized,
        ifeo_priority_locked,
        gpu_locked,
        raw_mouse_active,
        low_latency_net_active,
        hypervisor_status,
        hypervisor_off,
        hvci_disabled: !hvci,
        cache_bytes,
    }
}

/// Parses comma-separated `bst.installed_images="Nougat32,Pie64,Rvc64"` AND scans `bst.instance.<name>.*`
/// keys so every configured instance is handled cleanly without duplicates.
pub fn parse_bluestacks_instances(conf_text: &str) -> Vec<String> {
    let clean = conf_text.strip_prefix('\u{FEFF}').unwrap_or(conf_text);
    let mut seen = BTreeSet::new();
    let mut ordered = Vec::new();

    for line in clean.lines() {
        let trimmed = line.trim();
        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        let key = k.trim();
        let val = v.trim().trim_matches('"');

        if key == "bst.installed_images" {
            for part in val.split(',') {
                let name = part.trim().trim_matches('"');
                if !name.is_empty() && seen.insert(name.to_string()) {
                    ordered.push(name.to_string());
                }
            }
        } else if let Some(rest) = key.strip_prefix("bst.instance.") {
            if let Some((inst, _)) = rest.split_once('.') {
                let name = inst.trim();
                if !name.is_empty() && seen.insert(name.to_string()) {
                    ordered.push(name.to_string());
                }
            }
        }
    }

    ordered
}

fn audit_bluestacks_conf(raw: &str) -> (bool, bool, bool) {
    let mut high_fps = false;
    let mut rog = false;
    let mut astc_hw = false;
    let mut ads_off = false;

    for line in raw.lines() {
        let trimmed = line.trim();
        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        let key = k.trim();
        let val = v.trim().trim_matches('"');

        if key.ends_with(".max_fps") {
            if let Ok(fps) = val.parse::<u32>() {
                if fps >= 120 {
                    high_fps = true;
                }
            }
        }
        // Built-in ROG 2 preset code in BlueStacks/MSI is "rogt"; custom ROG 5 is "ASUS_I005DA"
        if (key.ends_with(".device_profile_code")
            && (val.eq_ignore_ascii_case("rogt") || val.eq_ignore_ascii_case("ASUS_I005DA")))
            || (key.ends_with(".device_custom_model") && val.eq_ignore_ascii_case("ASUS_I005DA"))
        {
            rog = true;
        }
        if key.ends_with(".astc_decoding_mode") && val.eq_ignore_ascii_case("hardware") {
            astc_hw = true;
        }
        if key == "bst.enable_programmatic_ads" && val == "0" {
            ads_off = true;
        }
    }
    (high_fps, rog, astc_hw && ads_off)
}

fn upsert_kv(
    kv: &mut BTreeMap<String, String>,
    existing_order: &mut Vec<String>,
    key: String,
    val: String,
) {
    if !kv.contains_key(&key) {
        existing_order.push(key.clone());
    }
    kv.insert(key, val);
}

/// Applies verified BlueStacks 5 / MSI App Player 5 (`5.12+`) config transforms to `conf_text`:
/// - Unlocks 240 FPS (`enable_high_fps="1"`, `max_fps="240"`, `enable_vsync="0"`)
/// - Sets valid ASUS ROG profile (`device_profile_code="rogt"` or custom `ASUS_I005DA` with `device_custom_manufacturer="ASUS"`)
/// - Enables hardware ASTC texture decoding (`astc_decoding_mode="hardware"`), `prefer_dedicated_gpu="1"`, `enable_mem_dedup="1"`
/// - Disables real BlueStacks/MSI ad & telemetry keys (`bst.enable_programmatic_ads="0"`, `bst.feature.programmatic_ads="0"`, `bst.feature.app_install_stats="0"`, `bst.feature.usage_stats="0"`, `bst.feature.bluestacksX="0"`, `bst.log_levels="*:E"`)
/// - Removes any accidental malformed `"enable_ root_access"` key with space
/// - Preserves `cpus >= 4` and bumps `ram` to `>= 4096` MB
pub fn apply_bluestacks_conf_transforms(
    conf_text: &str,
    enable_240fps: bool,
    enable_rog: bool,
) -> String {
    let clean = conf_text.strip_prefix('\u{FEFF}').unwrap_or(conf_text);
    let mut instances = parse_bluestacks_instances(clean);
    if instances.is_empty() {
        instances.push("Pie64".to_string());
    }

    let mut kv: BTreeMap<String, String> = BTreeMap::new();
    let mut existing_order: Vec<String> = Vec::new();
    let mut other_lines: Vec<String> = Vec::new();

    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            other_lines.push(line.to_string());
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            let key = k.trim().to_string();
            // Strip any malformed key containing internal whitespace (such as legacy "enable_ root_access")
            if key.contains(' ') {
                continue;
            }
            let val = v.trim().to_string();
            if !kv.contains_key(&key) {
                existing_order.push(key.clone());
            }
            kv.insert(key, val);
        } else {
            other_lines.push(line.to_string());
        }
    }

    if enable_240fps {
        // Real verified global performance & ad/telemetry keys from MSI App Player 5.12 / BlueStacks 5
        upsert_kv(&mut kv, &mut existing_order, "bst.prefer_dedicated_gpu".into(), "\"1\"".into());
        upsert_kv(&mut kv, &mut existing_order, "bst.enable_mem_dedup".into(), "\"1\"".into());
        upsert_kv(&mut kv, &mut existing_order, "bst.enable_programmatic_ads".into(), "\"0\"".into());
        upsert_kv(&mut kv, &mut existing_order, "bst.feature.programmatic_ads".into(), "\"0\"".into());
        upsert_kv(&mut kv, &mut existing_order, "bst.feature.app_install_stats".into(), "\"0\"".into());
        upsert_kv(&mut kv, &mut existing_order, "bst.feature.usage_stats".into(), "\"0\"".into());
        upsert_kv(&mut kv, &mut existing_order, "bst.feature.bluestacksX".into(), "\"0\"".into());
        upsert_kv(&mut kv, &mut existing_order, "bst.feature.nowbux".into(), "\"0\"".into());
        upsert_kv(&mut kv, &mut existing_order, "bst.feature.send_notification_stats".into(), "\"0\"".into());
        upsert_kv(&mut kv, &mut existing_order, "bst.log_levels".into(), "\"*:E\"".into());
    }

    for inst in &instances {
        if enable_240fps {
            upsert_kv(&mut kv, &mut existing_order, format!("bst.instance.{inst}.enable_high_fps"), "\"1\"".into());
            upsert_kv(&mut kv, &mut existing_order, format!("bst.instance.{inst}.max_fps"), "\"240\"".into());
            upsert_kv(&mut kv, &mut existing_order, format!("bst.instance.{inst}.enable_vsync"), "\"0\"".into());
            upsert_kv(&mut kv, &mut existing_order, format!("bst.instance.{inst}.astc_decoding_mode"), "\"hardware\"".into());
            upsert_kv(&mut kv, &mut existing_order, format!("bst.instance.{inst}.enable_logcat_redirection"), "\"0\"".into());
        }

        if enable_rog {
            // Use native BlueStacks/MSI "rogt" profile code AND populate custom ROG fields so both built-in and custom checks pass
            upsert_kv(
                &mut kv,
                &mut existing_order,
                format!("bst.instance.{inst}.device_profile_code"),
                "\"rogt\"".into(),
            );
            upsert_kv(
                &mut kv,
                &mut existing_order,
                format!("bst.instance.{inst}.device_custom_brand"),
                "\"ASUS\"".into(),
            );
            upsert_kv(
                &mut kv,
                &mut existing_order,
                format!("bst.instance.{inst}.device_custom_manufacturer"),
                "\"ASUS\"".into(),
            );
            upsert_kv(
                &mut kv,
                &mut existing_order,
                format!("bst.instance.{inst}.device_custom_model"),
                "\"ASUS_I005DA\"".into(),
            );
        }

        // Preserve user's higher CPU allocation (`>= 4`); bump only if < 4 or unset
        let cpu_key = format!("bst.instance.{inst}.cpus");
        let cur_cpus = kv
            .get(&cpu_key)
            .and_then(|v| v.trim().trim_matches('"').parse::<u32>().ok())
            .unwrap_or(0);
        if cur_cpus < 4 {
            upsert_kv(&mut kv, &mut existing_order, cpu_key, "\"4\"".into());
        }

        // Preserve user's higher RAM allocation (`>= 4096`, e.g. 12288 on this PC!); bump only if < 4096
        let ram_key = format!("bst.instance.{inst}.ram");
        let cur_ram = kv
            .get(&ram_key)
            .and_then(|v| v.trim().trim_matches('"').parse::<u32>().ok())
            .unwrap_or(0);
        if cur_ram < 4096 {
            upsert_kv(&mut kv, &mut existing_order, ram_key, "\"4096\"".into());
        }
    }

    let mut out_lines: Vec<String> = Vec::with_capacity(existing_order.len() + other_lines.len());
    for l in other_lines {
        out_lines.push(l);
    }
    for k in existing_order {
        if let Some(v) = kv.get(&k) {
            out_lines.push(format!("{k}={v}"));
        }
    }
    let mut joined = out_lines.join("\r\n");
    joined.push_str("\r\n");
    joined
}

/// Cleanly stops all BlueStacks/MSI processes (`HD-Player.exe`, `HD-MultiInstanceManager.exe`, `BlueStacksHelper.exe`, `BstkVMMgr.exe`, AND `BstkSVC.exe`)
/// before patching `bluestacks.conf` so `BstkSVC.exe` never overwrites the patched file on exit.
pub fn patch_bluestacks_conf(
    conf_path: &Path,
    enable_240fps: bool,
    enable_rog: bool,
    journal: &mut TransactionJournal,
) -> Result<(), String> {
    let bs_procs = [
        "HD-Player.exe",
        "HD-MultiInstanceManager.exe",
        "BlueStacksHelper.exe",
        "BstkVMMgr.exe",
        "BstkSVC.exe",
    ];

    if is_process_running_ci("HD-Player")
        || is_process_running_ci("BstkSVC")
        || is_process_running_ci("BstkVMMgr")
    {
        // Step 1: Graceful close request for player & manager windows
        for p in ["HD-Player.exe", "HD-MultiInstanceManager.exe", "BlueStacksHelper.exe"] {
            let _ = run_command_timeout("taskkill.exe", &["/IM", p], 2500);
        }
        std::thread::sleep(std::time::Duration::from_millis(600));

        // Step 2: Terminate remaining background COM service (BstkSVC.exe / BstkVMMgr.exe) so it flushes before our patch
        for p in &bs_procs {
            let _ = run_command_timeout("taskkill.exe", &["/F", "/IM", p], 2500);
        }

        for _ in 0..10 {
            if !is_process_running_ci("HD-Player") && !is_process_running_ci("BstkSVC") {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
        }

        if is_process_running_ci("HD-Player") || is_process_running_ci("BstkSVC") {
            return Err(
                "BlueStacks / MSI App Player (HD-Player.exe or BstkSVC.exe) is still running. Close all emulator windows and retry."
                    .to_string(),
            );
        }
    }

    let existing = std::fs::read_to_string(conf_path)
        .map_err(|e| format!("Failed reading {}: {e}", conf_path.display()))?;
    let transformed = apply_bluestacks_conf_transforms(&existing, enable_240fps, enable_rog);
    atomic_write_utf8_no_bom(conf_path, &transformed, journal)
}

fn find_ldplayer_configs(ld_exe: Option<&str>) -> Vec<String> {
    let Some(exe) = ld_exe else {
        return Vec::new();
    };
    let Some(dir) = Path::new(exe).parent() else {
        return Vec::new();
    };
    let cfg_dir = dir.join(r"vms\config");
    let Ok(entries) = std::fs::read_dir(cfg_dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for ent in entries.flatten() {
        let p = ent.path();
        let fname = ent.file_name().to_string_lossy().to_ascii_lowercase();
        if p.is_file()
            && fname.starts_with("leidian")
            && fname.ends_with(".config")
            && fname != "leidian.config"
        {
            out.push(p.to_string_lossy().to_string());
        }
    }
    out
}

/// Patches LDPlayer 9 instance JSON configs (`vms\config\leidian*.config`) including `"basicSettings.highFp": true`
/// (required by LDPlayer 9 to honor `basicSettings.fps = 240`), ROG Phone 5 (`ASUS_I005DA`), ASTC textures, and raw mouse.
pub fn patch_ldplayer_configs(
    configs: &[String],
    journal: &mut TransactionJournal,
) -> Result<usize, String> {
    if is_process_running_ci("dnplayer") || is_process_running_ci("LdVBoxHeadless") {
        return Err("Close LDPlayer 9 before patching leidian*.config files.".to_string());
    }
    if configs.is_empty() {
        return Ok(0);
    }
    let mut count = 0usize;
    for path_str in configs {
        let p = Path::new(path_str);
        let Ok(raw) = std::fs::read_to_string(p) else {
            continue;
        };
        let clean = raw.strip_prefix('\u{FEFF}').unwrap_or(&raw);
        let mut json: serde_json::Value =
            serde_json::from_str(clean).unwrap_or_else(|_| serde_json::json!({}));
        if let Some(obj) = json.as_object_mut() {
            obj.insert("basicSettings.highFp".into(), serde_json::json!(true));
            obj.insert("basicSettings.fps".into(), serde_json::json!(240));
            obj.insert("basicSettings.astc".into(), serde_json::json!(true));
            obj.insert("basicSettings.hdrQuality".into(), serde_json::json!(0));
            obj.insert("basicSettings.disableMouseFast".into(), serde_json::json!(true));
            obj.insert("propertySettings.phoneManufacturer".into(), serde_json::json!("ASUS"));
            obj.insert("propertySettings.phoneModel".into(), serde_json::json!("ASUS_I005DA"));
            let cur_cpu = obj
                .get("advancedSettings.cpuCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            if cur_cpu < 4 {
                obj.insert("advancedSettings.cpuCount".into(), serde_json::json!(4));
            }
            let cur_mem = obj
                .get("advancedSettings.memorySize")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            if cur_mem < 4096 {
                obj.insert("advancedSettings.memorySize".into(), serde_json::json!(4096));
            }
        }
        let formatted = serde_json::to_string_pretty(&json)
            .map_err(|e| format!("Failed serializing LDPlayer config: {e}"))?;
        atomic_write_utf8_no_bom(p, &formatted, journal)?;
        count += 1;
    }
    Ok(count)
}

/// Registers Kernel IFEO `PerfOptions` (`CpuPriorityClass=3` High, `PagePriority=5` Normal-Max)
/// on render/VM engines (`HD-Player.exe`, `BstkVMMgr.exe`, `dnplayer.exe`, `LdVBoxHeadless.exe`, etc.).
/// Note: Does NOT set `IoPriority=3` (which Windows NT `PerfOptions` rejects in user-mode) and does NOT boost `BstkSVC.exe`.
pub fn apply_emulator_ifeo_priority_lock(
    journal: &mut TransactionJournal,
    enable: bool,
) -> Result<usize, String> {
    let mut count = 0usize;
    for exe in EMULATOR_RENDER_IFEO_EXES {
        let sub = format!(r"{IFEO_BASE_SUBKEY}\{exe}\PerfOptions");
        if enable {
            reg_set_journaled(
                journal,
                RegHive::Hklm,
                &sub,
                "CpuPriorityClass",
                TypedRegValue::Dword(3),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hklm,
                &sub,
                "PagePriority",
                TypedRegValue::Dword(5),
            )?;
            let _ = reg_delete_value_journaled(journal, RegHive::Hklm, &sub, "IoPriority");
        } else {
            let _ = reg_delete_value_journaled(journal, RegHive::Hklm, &sub, "CpuPriorityClass");
            let _ = reg_delete_value_journaled(journal, RegHive::Hklm, &sub, "PagePriority");
            let _ = reg_delete_value_journaled(journal, RegHive::Hklm, &sub, "IoPriority");
        }
        count += 1;
    }
    // Clean up any legacy BstkSVC.exe IFEO entry if previously created
    let svc_sub = format!(r"{IFEO_BASE_SUBKEY}\BstkSVC.exe\PerfOptions");
    let _ = reg_delete_value_journaled(journal, RegHive::Hklm, &svc_sub, "CpuPriorityClass");
    let _ = reg_delete_value_journaled(journal, RegHive::Hklm, &svc_sub, "IoPriority");
    let _ = reg_delete_value_journaled(journal, RegHive::Hklm, &svc_sub, "PagePriority");

    Ok(count)
}

fn existing_bs_render_exes(product_folder: &str) -> Vec<String> {
    let mut found = Vec::new();
    let (_, reg_install_dir, _, _) = query_bluestacks_variant_paths(product_folder);
    let mut dirs = vec![reg_install_dir];
    if let Ok(pf) = std::env::var("ProgramFiles") {
        dirs.push(Path::new(&pf).join(product_folder));
    }
    if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
        dirs.push(Path::new(&pf86).join(product_folder));
    }
    dirs.push(PathBuf::from(format!(r"C:\Program Files\{product_folder}")));
    dirs.push(PathBuf::from(format!(r"D:\Program Files\{product_folder}")));

    // Only target actual GPU/VM executables (HD-Player.exe and BstkVMMgr.exe), not HD-Adb.exe
    for base in dirs {
        for exe_name in ["HD-Player.exe", "BstkVMMgr.exe"] {
            let p = base.join(exe_name);
            if p.is_file() {
                let s = p.to_string_lossy().to_string();
                if !found.iter().any(|x: &String| x.eq_ignore_ascii_case(&s)) {
                    found.push(s);
                }
            }
        }
    }
    found
}

fn detect_ldplayer_exe() -> Option<String> {
    for p in [
        r"C:\LDPlayer\LDPlayer9\dnplayer.exe",
        r"D:\LDPlayer\LDPlayer9\dnplayer.exe",
        r"E:\LDPlayer\LDPlayer9\dnplayer.exe",
    ] {
        if Path::new(p).is_file() {
            return Some(p.to_string());
        }
    }
    for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        if let Ok(k) = RegKey::predef(hive)
            .open_subkey_with_flags(r"SOFTWARE\XuanZhi\LDPlayer9", KEY_READ | KEY_WOW64_64KEY)
        {
            if let Ok(dir) = k.get_value::<String, _>("InstallDir") {
                let exe = Path::new(&dir).join("dnplayer.exe");
                if exe.is_file() {
                    return Some(exe.to_string_lossy().to_string());
                }
            }
        }
    }
    None
}

fn collect_existing_emulator_render_exes() -> Vec<String> {
    let mut out = Vec::new();
    out.extend(existing_bs_render_exes("BlueStacks_nxt"));
    out.extend(existing_bs_render_exes("BlueStacks_msi5"));
    if let Some(ld) = detect_ldplayer_exe() {
        if let Some(dir) = Path::new(&ld).parent() {
            let vbox = dir.join("LdVBoxHeadless.exe");
            if vbox.is_file() {
                out.push(vbox.to_string_lossy().to_string());
            }
        }
        out.push(ld);
    }
    out
}

/// Strictly registers High-Performance dGPU (`GpuPreference=2;`) AND Fullscreen Optimization / High-DPI Override
/// (`~ DISABLEDXMAXIMIZEDWINDOWEDMODE HIGHDPIAWARE`) ONLY for existing installed emulator render executables.
pub fn apply_gpu_preference_lock(journal: &mut TransactionJournal) -> Result<usize, String> {
    let exes = collect_existing_emulator_render_exes();
    if exes.is_empty() {
        return Err("No installed Android emulator executables (HD-Player.exe / dnplayer.exe) found."
            .to_string());
    }
    for exe in &exes {
        reg_set_journaled(
            journal,
            RegHive::Hkcu,
            GPU_PREF_SUBKEY,
            exe,
            TypedRegValue::String("GpuPreference=2;".to_string()),
        )?;
        reg_set_journaled(
            journal,
            RegHive::Hkcu,
            APP_COMPAT_LAYERS,
            exe,
            TypedRegValue::String("~ DISABLEDXMAXIMIZEDWINDOWEDMODE HIGHDPIAWARE".to_string()),
        )?;
    }
    Ok(exes.len())
}

/// Applies true 1:1 mouse precision via `SPI_SETMOUSE` without overwriting `MouseSensitivity`.
pub fn apply_raw_mouse_aim(
    journal: &mut TransactionJournal,
    enable_raw: bool,
) -> Result<(), String> {
    apply_mouse_acceleration(journal, !enable_raw)
}

/// Disables Nagle (`TCPNoDelay=1`, `TcpAckFrequency=1`) & applies MMCSS optimization (never deletes keys when enabling).
pub fn apply_low_latency_net(
    journal: &mut TransactionJournal,
    enable: bool,
) -> Result<(), String> {
    apply_mmcss_network_throttle(journal, enable)?;
    apply_nagle_on_primary(journal, enable)
}

pub fn set_hypervisor_state(turn_off: bool, journal: &mut TransactionJournal) -> Result<String, String> {
    let val = if turn_off { 0u32 } else { 1u32 };
    let _ = reg_set_journaled(
        journal,
        RegHive::Hklm,
        r"SYSTEM\CurrentControlSet\Control\DeviceGuard\Scenarios\HypervisorEnforcedCodeIntegrity",
        "Enabled",
        TypedRegValue::Dword(val),
    );
    let _ = reg_set_journaled(
        journal,
        RegHive::Hklm,
        r"SYSTEM\CurrentControlSet\Control\DeviceGuard",
        "EnableVirtualizationBasedSecurity",
        TypedRegValue::Dword(val),
    );

    let mode = if turn_off { "off" } else { "auto" };
    let out = run_command_timeout("bcdedit.exe", &["/set", "hypervisorlaunchtype", mode], 5000)?;
    if out.exit_code == 0 {
        Ok(format!(
            "BCD hypervisorlaunchtype={mode} & HVCI={} applied. Reboot PC for raw VT-x/AMD-V emulator engine.",
            if turn_off { "OFF" } else { "ON" }
        ))
    } else {
        let msg = format!("{}\n{}", out.stdout.trim(), out.stderr.trim());
        Err(format!("bcdedit failed: {}", msg.trim()))
    }
}

fn emulator_log_specs() -> Vec<CleanerTargetSpec> {
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let (_, _, _, bs_log_dir) = query_bluestacks_variant_paths("BlueStacks_nxt");
    let (_, _, _, msi_log_dir) = query_bluestacks_variant_paths("BlueStacks_msi5");

    let mut specs = vec![
        CleanerTargetSpec {
            base_path: bs_log_dir.to_string_lossy().to_string(),
            wildcard: None,
            min_age_hours: None,
        },
        CleanerTargetSpec {
            base_path: msi_log_dir.to_string_lossy().to_string(),
            wildcard: None,
            min_age_hours: None,
        },
        CleanerTargetSpec {
            base_path: Path::new(&local)
                .join(r"BlueStacks_nxt\Logs")
                .to_string_lossy()
                .to_string(),
            wildcard: None,
            min_age_hours: None,
        },
    ];
    if let Some(ld) = detect_ldplayer_exe() {
        if let Some(dir) = Path::new(&ld).parent() {
            specs.push(CleanerTargetSpec {
                base_path: dir.join("Logs").to_string_lossy().to_string(),
                wildcard: None,
                min_age_hours: None,
            });
            specs.push(CleanerTargetSpec {
                base_path: dir.join(r"vms\customizeConfigs").to_string_lossy().to_string(),
                wildcard: Some("*.log".to_string()),
                min_age_hours: None,
            });
        }
    }
    specs
}

pub fn purge_emulator_logs_safe() -> (u64, usize, usize) {
    let specs = emulator_log_specs();
    purge_specs_safe(&specs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bluestacks_multi_instance_parser_tests() {
        let raw = "\
            bst.installed_images=\"Nougat32,Pie64, Android11,Pie64\"\r\n\
            bst.enable_programmatic_ads=\"1\"\r\n\
            bst.instance.Pie64.cpus=\"6\"\r\n\
            bst.instance.Pie64.ram=\"12288\"\r\n\
            bst.instance.Pie64.device_profile_code=\"rogt\"\r\n\
            bst.instance.Pie64.enable_ root_access=\"0\"\r\n\
            bst.instance.Nougat32.cpus=\"2\"\r\n\
            bst.instance.Nougat32.ram=\"2048\"\r\n\
            bst.instance.Android11.display_name=\"A11\"\r\n";

        let instances = parse_bluestacks_instances(raw);
        assert_eq!(instances, vec!["Nougat32", "Pie64", "Android11"]);

        // Verify built-in "rogt" profile code is recognized by audit_bluestacks_conf!
        let (_, rog_before, _) = audit_bluestacks_conf(raw);
        assert!(rog_before, "Built-in 'rogt' profile code must be recognized as ROG active");

        let transformed = apply_bluestacks_conf_transforms(raw, true, true);
        // Malformed "enable_ root_access" with space must be stripped!
        assert!(!transformed.contains("enable_ root_access"));
        // Existing Pie64 cpus=6 and ram=12288 MUST be preserved!
        assert!(transformed.contains("bst.instance.Pie64.cpus=\"6\""));
        assert!(transformed.contains("bst.instance.Pie64.ram=\"12288\""));
        // Nougat32 cpus=2 -> 4 and ram=2048 -> 4096!
        assert!(transformed.contains("bst.instance.Nougat32.cpus=\"4\""));
        assert!(transformed.contains("bst.instance.Nougat32.ram=\"4096\""));
        // Unset Android11 cpus=4 and ram=4096!
        assert!(transformed.contains("bst.instance.Android11.cpus=\"4\""));
        assert!(transformed.contains("bst.instance.Android11.ram=\"4096\""));
        // 240 FPS + rogt / ASUS_I005DA + ASTC hardware + programmatic ads off
        assert!(transformed.contains("bst.instance.Pie64.max_fps=\"240\""));
        assert!(transformed.contains("bst.instance.Pie64.astc_decoding_mode=\"hardware\""));
        assert!(transformed.contains("bst.enable_programmatic_ads=\"0\""));
        assert!(transformed.contains("bst.instance.Android11.device_profile_code=\"rogt\""));
        assert!(transformed.contains("bst.instance.Android11.device_custom_model=\"ASUS_I005DA\""));

        let (fps_after, rog_after, astc_ads_after) = audit_bluestacks_conf(&transformed);
        assert!(fps_after && rog_after && astc_ads_after);
    }
}
