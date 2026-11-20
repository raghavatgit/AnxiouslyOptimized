use crate::engine::cleaner::{collect_matching_files, purge_specs_safe};
use crate::engine::network::icmp_ping_ipv4;
use crate::engine::safety::{
    atomic_write_utf8_no_bom, read_reg_dword, read_reg_string, reg_create_key_and_set_journaled,
    reg_delete_value_journaled, reg_set_journaled,
};
use crate::engine::tweaks::{
    activate_performance_power_plan, apply_mmcss_network_throttle, apply_mouse_acceleration,
    apply_nagle_on_primary, check_is_high_or_ultimate_plan, check_mmcss_optimized,
    check_nagle_disabled_on_primary, read_active_power_ac_val, write_active_power_ac_val_journaled,
    SUB_USB_GUID, USB_SUSPEND_GUID,
};
use crate::models::{
    CleanerTargetSpec, RegHive, RiotRegionPing, TransactionJournal, TypedRegValue,
    ValorantInstallStatus, ValorantPresetKind, ValorantTopology,
};
use crate::system::ecosystem::{detect_valorant_quick, is_process_running_ci};
use rayon::prelude::*;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const APP_COMPAT_LAYERS: &str =
    r"Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers";
const QOS_VAL_SUBKEY: &str = r"SOFTWARE\Policies\Microsoft\Windows\QoS\OptimizerByRaghav_Valorant";

pub fn detect_topology() -> ValorantTopology {
    let (status, install_path, riot_client_path) = detect_valorant_quick();
    let shipping_exe_path = install_path.as_ref().and_then(|dir| {
        let p = Path::new(dir).join(r"ShooterGame\Binaries\Win64\VALORANT-Win64-Shipping.exe");
        if p.exists() {
            Some(p.to_string_lossy().to_string())
        } else {
            None
        }
    });

    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let config_paths = find_valorant_config_inis(&local);
    let active_config_path = config_paths.first().cloned();

    let (resolution_quality, vsync_disabled) = active_config_path
        .as_deref()
        .map(parse_valorant_ini_summary)
        .unwrap_or((100, false));

    let (fso_disabled, dpi_aware) = shipping_exe_path
        .as_deref()
        .and_then(|exe| read_reg_string(RegHive::Hkcu, APP_COMPAT_LAYERS, exe))
        .map(|val| {
            let up = val.to_ascii_uppercase();
            (
                up.contains("DISABLEDXMAXIMIZEDWINDOWEDMODE"),
                up.contains("HIGHDPIAWARE"),
            )
        })
        .unwrap_or((false, false));

    let game_dvr_disabled = read_reg_dword(
        RegHive::Hkcu,
        r"Software\Microsoft\Windows\CurrentVersion\GameDVR",
        "AppCaptureEnabled",
    ) == Some(0);

    let qos_policy_exists =
        read_reg_string(RegHive::Hklm, QOS_VAL_SUBKEY, "DSCP Value").as_deref() == Some("46");

    let mouse_accel_disabled = read_reg_string(RegHive::Hkcu, r"Control Panel\Mouse", "MouseSpeed")
        .as_deref()
        == Some("0")
        && read_reg_string(RegHive::Hkcu, r"Control Panel\Mouse", "MouseThreshold1").as_deref()
            == Some("0")
        && read_reg_string(RegHive::Hkcu, r"Control Panel\Mouse", "MouseThreshold2").as_deref()
            == Some("0");

    let usb_suspend_disabled = read_active_power_ac_val(SUB_USB_GUID, USB_SUSPEND_GUID) == Some(0);

    let (shader_cache_bytes, log_cache_bytes, crash_cache_bytes) = if !local.is_empty() {
        let base = Path::new(&local).join(r"VALORANT\Saved");
        let sum_dir = |sub: &str| -> u64 {
            let p = base.join(sub);
            if !p.exists() {
                return 0;
            }
            let specs = vec![CleanerTargetSpec {
                base_path: p.to_string_lossy().to_string(),
                wildcard: None,
                min_age_hours: None,
            }];
            collect_matching_files(&specs).iter().map(|(_, b)| *b).sum()
        };
        (
            sum_dir("ShaderCache") + sum_dir("PipelineCaches"),
            sum_dir("Logs"),
            sum_dir("Crashes"),
        )
    } else {
        (0, 0, 0)
    };

    ValorantTopology {
        install_status: status,
        install_path,
        riot_client_path,
        shipping_exe_path,
        config_paths,
        active_config_path,
        resolution_quality,
        vsync_disabled,
        fso_disabled,
        dpi_aware,
        game_dvr_disabled,
        nagle_disabled: check_nagle_disabled_on_primary(),
        mmcss_optimized: check_mmcss_optimized(),
        qos_policy_exists,
        mouse_accel_disabled,
        usb_suspend_disabled,
        power_high_perf: check_is_high_or_ultimate_plan(),
        shader_cache_bytes,
        log_cache_bytes,
        crash_cache_bytes,
    }
}

fn find_valorant_config_inis(local_app_data: &str) -> Vec<String> {
    if local_app_data.is_empty() {
        return Vec::new();
    }
    let cfg_root = Path::new(local_app_data).join(r"VALORANT\Saved\Config");
    let Ok(entries) = std::fs::read_dir(cfg_root) else {
        return Vec::new();
    };

    let mut found: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    for ent in entries.flatten() {
        let dir = ent.path();
        if !dir.is_dir() {
            continue;
        }
        // Account UUID folders (plus Windows / WindowsClient)
        for sub in ["Windows", "WindowsClient"] {
            let ini = dir.join(sub).join("GameUserSettings.ini");
            if ini.is_file() {
                let mtime = std::fs::metadata(&ini)
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::UNIX_EPOCH);
                found.push((mtime, ini));
            }
        }
        // Direct fallback if `Windows` folder itself holds INI
        let direct_ini = dir.join("GameUserSettings.ini");
        if direct_ini.is_file() {
            let mtime = std::fs::metadata(&direct_ini)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            found.push((mtime, direct_ini));
        }
    }

    found.sort_by(|a, b| b.0.cmp(&a.0));
    found
        .into_iter()
        .map(|(_, p)| p.to_string_lossy().to_string())
        .collect()
}

fn parse_valorant_ini_summary(path: &str) -> (u32, bool) {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return (100, false);
    };
    let clean = raw.strip_prefix('\u{FEFF}').unwrap_or(&raw);
    let mut res_q = 100u32;
    let mut vsync_off = false;
    for line in clean.lines() {
        let trimmed = line.trim();
        if let Some((k, v)) = trimmed.split_once('=') {
            let k_low = k.trim().to_ascii_lowercase();
            let v_clean = v.trim();
            if k_low == "sg.resolutionquality" {
                if let Ok(f) = v_clean.parse::<f32>() {
                    res_q = f.round().clamp(50.0, 200.0) as u32;
                }
            } else if k_low == "busevsync" {
                vsync_off = v_clean.eq_ignore_ascii_case("false") || v_clean == "0";
            }
        }
    }
    (res_q, vsync_off)
}

pub fn apply_fso_dpi(
    shipping_exe: &str,
    enable: bool,
    journal: &mut TransactionJournal,
) -> Result<(), String> {
    if shipping_exe.trim().is_empty() {
        return Err("VALORANT-Win64-Shipping.exe path not detected".to_string());
    }
    if enable {
        reg_set_journaled(
            journal,
            RegHive::Hkcu,
            APP_COMPAT_LAYERS,
            shipping_exe,
            TypedRegValue::String("~ DISABLEDXMAXIMIZEDWINDOWEDMODE HIGHDPIAWARE".to_string()),
        )
    } else {
        reg_delete_value_journaled(journal, RegHive::Hkcu, APP_COMPAT_LAYERS, shipping_exe)
    }
}

pub fn apply_qos_dscp(enable: bool, journal: &mut TransactionJournal) -> Result<(), String> {
    if enable {
        let pairs = [
            ("Version", "1.0"),
            ("Application Name", "VALORANT-Win64-Shipping.exe"),
            ("Protocol", "*"),
            ("Local Port", "*"),
            ("Local IP", "*"),
            ("Local IP Prefix Length", "*"),
            ("Remote Port", "*"),
            ("Remote IP", "*"),
            ("Remote IP Prefix Length", "*"),
            ("DSCP Value", "46"),
            ("Throttle Rate", "-1"),
        ];
        for (name, val) in pairs {
            reg_create_key_and_set_journaled(
                journal,
                RegHive::Hklm,
                QOS_VAL_SUBKEY,
                QOS_VAL_SUBKEY,
                name,
                TypedRegValue::String(val.to_string()),
            )?;
        }
        Ok(())
    } else {
        for name in [
            "DSCP Value",
            "Application Name",
            "Throttle Rate",
            "Version",
            "Protocol",
        ] {
            let _ = reg_delete_value_journaled(journal, RegHive::Hklm, QOS_VAL_SUBKEY, name);
        }
        Ok(())
    }
}

/// Pure testable INI updater:
/// - Strips UTF-8 BOM if present
/// - Preserves all comments & unrelated keys
/// - Updates existing keys under matching `[section]`
/// - Inserts missing keys before the next `[section]` (or appends missing `[section]` cleanly)
pub fn patch_ini_content(raw: &str, updates: &[(&str, &str, &str)]) -> String {
    let clean = raw.strip_prefix('\u{FEFF}').unwrap_or(raw);
    let mut lines: Vec<String> = clean.lines().map(|l| l.trim_end_matches('\r').to_string()).collect();

    for &(target_section, target_key, target_val) in updates {
        let mut current_section = String::new();
        let mut section_found = false;
        let mut section_end_idx = lines.len();
        let mut updated = false;

        for (i, line) in lines.iter_mut().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                if section_found && !updated {
                    section_end_idx = i;
                    break;
                }
                current_section = trimmed[1..trimmed.len() - 1].to_string();
                if current_section.eq_ignore_ascii_case(target_section) {
                    section_found = true;
                }
                continue;
            }

            if section_found && current_section.eq_ignore_ascii_case(target_section) {
                if let Some((k, _)) = trimmed.split_once('=') {
                    if k.trim().eq_ignore_ascii_case(target_key) {
                        *line = format!("{target_key}={target_val}");
                        updated = true;
                        break;
                    }
                }
            }
        }

        if !updated {
            if section_found {
                // Insert right before next section (above any blank trailing line if present)
                let mut insert_at = section_end_idx;
                while insert_at > 0 && lines[insert_at - 1].trim().is_empty() {
                    insert_at -= 1;
                }
                lines.insert(insert_at, format!("{target_key}={target_val}"));
            } else {
                if !lines.is_empty() && !lines.last().map(|l| l.is_empty()).unwrap_or(true) {
                    lines.push(String::new());
                }
                lines.push(format!("[{target_section}]"));
                lines.push(format!("{target_key}={target_val}"));
            }
        }
    }

    let mut out = lines.join("\r\n");
    out.push_str("\r\n");
    out
}

pub fn patch_ini_no_bom(
    ini_path: &Path,
    updates: &[(&str, &str, &str)],
    journal: &mut TransactionJournal,
) -> Result<(), String> {
    if is_process_running_ci("VALORANT-Win64-Shipping") || is_process_running_ci("VALORANT") {
        return Err(
            "Close VALORANT before patching GameUserSettings.ini so changes are not overwritten on game exit."
                .to_string(),
        );
    }
    let existing = if ini_path.exists() {
        std::fs::read_to_string(ini_path)
            .map_err(|e| format!("Failed reading {}: {e}", ini_path.display()))?
    } else {
        String::new()
    };
    let patched = patch_ini_content(&existing, updates);
    atomic_write_utf8_no_bom(ini_path, &patched, journal)
}

pub fn apply_preset(
    topo: &ValorantTopology,
    kind: ValorantPresetKind,
    journal: &mut TransactionJournal,
) -> Result<usize, String> {
    if topo.config_paths.is_empty() {
        return Err(
            "No Valorant GameUserSettings.ini files found. Launch Valorant once to generate your account config."
                .to_string(),
        );
    }

    let updates: Vec<(&str, &str, &str)> = match kind {
        ValorantPresetKind::CompetitivePotato => vec![
            ("ScalabilityGroups", "sg.ResolutionQuality", "75.000000"),
            ("ScalabilityGroups", "sg.ViewDistanceQuality", "0"),
            ("ScalabilityGroups", "sg.AntiAliasingQuality", "0"),
            ("ScalabilityGroups", "sg.ShadowQuality", "0"),
            ("ScalabilityGroups", "sg.PostProcessQuality", "0"),
            ("ScalabilityGroups", "sg.TextureQuality", "0"),
            ("ScalabilityGroups", "sg.EffectsQuality", "0"),
            ("ScalabilityGroups", "sg.FoliageQuality", "0"),
            ("ScalabilityGroups", "sg.ShadingQuality", "0"),
            ("/Script/ShooterGame.ShooterGameUserSettings", "bUseVSync", "False"),
        ],
        ValorantPresetKind::Tournament240Hz => vec![
            ("ScalabilityGroups", "sg.ResolutionQuality", "100.000000"),
            ("ScalabilityGroups", "sg.ViewDistanceQuality", "1"),
            ("ScalabilityGroups", "sg.AntiAliasingQuality", "0"),
            ("ScalabilityGroups", "sg.ShadowQuality", "0"),
            ("ScalabilityGroups", "sg.PostProcessQuality", "0"),
            ("ScalabilityGroups", "sg.TextureQuality", "1"),
            ("ScalabilityGroups", "sg.EffectsQuality", "0"),
            ("ScalabilityGroups", "sg.ShadingQuality", "0"),
            ("/Script/ShooterGame.ShooterGameUserSettings", "bUseVSync", "False"),
        ],
        ValorantPresetKind::LaptopBalanced => vec![
            ("ScalabilityGroups", "sg.ResolutionQuality", "100.000000"),
            ("ScalabilityGroups", "sg.ViewDistanceQuality", "1"),
            ("ScalabilityGroups", "sg.AntiAliasingQuality", "1"),
            ("ScalabilityGroups", "sg.ShadowQuality", "1"),
            ("ScalabilityGroups", "sg.PostProcessQuality", "0"),
            ("ScalabilityGroups", "sg.TextureQuality", "1"),
            ("ScalabilityGroups", "sg.EffectsQuality", "1"),
            ("ScalabilityGroups", "sg.ShadingQuality", "0"),
            ("/Script/ShooterGame.ShooterGameUserSettings", "bUseVSync", "False"),
        ],
    };

    let mut count = 0usize;
    for path_str in &topo.config_paths {
        patch_ini_no_bom(Path::new(path_str), &updates, journal)?;
        count += 1;
    }
    Ok(count)
}

/// Applies safe OS-level latency, QoS DSCP 46, FSO/DPI, GameDVR, Mouse Accel, USB suspend & MMCSS tweaks.
/// Zero Vanguard or game memory/process interaction.
pub fn apply_competitive_bundle(
    topo: &ValorantTopology,
    journal: &mut TransactionJournal,
) -> Vec<String> {
    let mut notes = Vec::new();

    if let Some(ref exe) = topo.shipping_exe_path {
        match apply_fso_dpi(exe, true, journal) {
            Ok(()) => notes.push("[OK] Fullscreen Optimizations & DPI Override set for VALORANT-Win64-Shipping.exe".into()),
            Err(e) => notes.push(format!("[WARN] FSO/DPI skipped: {e}")),
        }
    } else {
        notes.push("[SKIP] VALORANT-Win64-Shipping.exe not detected; skipped AppCompatFlags".into());
    }

    match apply_qos_dscp(true, journal) {
        Ok(()) => notes.push("[OK] QoS DSCP 46 Expedited Forwarding policy registered".into()),
        Err(e) => notes.push(format!("[WARN] QoS DSCP skipped: {e}")),
    }

    let _ = reg_set_journaled(
        journal,
        RegHive::Hkcu,
        r"Software\Microsoft\Windows\CurrentVersion\GameDVR",
        "AppCaptureEnabled",
        TypedRegValue::Dword(0),
    );
    let _ = reg_set_journaled(
        journal,
        RegHive::Hkcu,
        r"Software\Microsoft\GameBar",
        "AllowAutoGameMode",
        TypedRegValue::Dword(1),
    );
    notes.push("[OK] Windows GameDVR background capture disabled".into());

    match apply_mouse_acceleration(journal, false) {
        Ok(()) => notes.push("[OK] Raw 1:1 mouse input applied live via SystemParametersInfoW".into()),
        Err(e) => notes.push(format!("[WARN] Mouse setting: {e}")),
    }

    match apply_mmcss_network_throttle(journal, true) {
        Ok(()) => notes.push("[OK] MMCSS NetworkThrottlingIndex & SystemResponsiveness tuned".into()),
        Err(e) => notes.push(format!("[WARN] MMCSS: {e}")),
    }

    match apply_nagle_on_primary(journal, true) {
        Ok(()) => notes.push("[OK] Primary adapter TCPNoDelay & TcpAckFrequency set".into()),
        Err(e) => notes.push(format!("[WARN] Nagle: {e}")),
    }

    let _ = write_active_power_ac_val_journaled(journal, SUB_USB_GUID, USB_SUSPEND_GUID, 0);
    match activate_performance_power_plan(journal) {
        Ok(()) => notes.push("[OK] Performance power scheme activated".into()),
        Err(e) => notes.push(format!("[WARN] Power scheme: {e}")),
    }

    notes
}

pub fn purge_caches_safe() -> Result<(u64, usize, usize), String> {
    if is_process_running_ci("VALORANT-Win64-Shipping") || is_process_running_ci("VALORANT") {
        return Err("Close VALORANT before cleaning shader, log, and crash caches.".to_string());
    }
    let local = std::env::var("LOCALAPPDATA")
        .map_err(|_| "LOCALAPPDATA environment variable missing".to_string())?;
    let base = Path::new(&local).join(r"VALORANT\Saved");
    let specs: Vec<CleanerTargetSpec> = ["Logs", "Crashes", "ShaderCache", "PipelineCaches"]
        .into_iter()
        .map(|sub| CleanerTargetSpec {
            base_path: base.join(sub).to_string_lossy().to_string(),
            wildcard: None,
            min_age_hours: None,
        })
        .collect();

    Ok(purge_specs_safe(&specs))
}

pub fn ping_riot_regions() -> Vec<RiotRegionPing> {
    // Public AWS / Riot regional edge endpoints representative of Riot shard POPs
    let targets = [
        ("Mumbai (AP-South)", "dynamodb.ap-south-1.amazonaws.com", Some("13.232.0.1")),
        ("Singapore (AP-Southeast)", "dynamodb.ap-southeast-1.amazonaws.com", Some("104.160.156.1")),
        ("Tokyo (AP-Northeast)", "dynamodb.ap-northeast-1.amazonaws.com", Some("104.160.155.3")),
        ("Sydney (OC1)", "dynamodb.ap-southeast-2.amazonaws.com", Some("104.160.141.3")),
        ("Frankfurt (EU-Central)", "dynamodb.eu-central-1.amazonaws.com", Some("104.160.136.3")),
        ("London (EU-West)", "dynamodb.eu-west-2.amazonaws.com", None),
        ("Virginia (US-East)", "dynamodb.us-east-1.amazonaws.com", Some("162.249.72.1")),
        ("Oregon (US-West)", "dynamodb.us-west-2.amazonaws.com", None),
        ("Sao Paulo (BR1)", "dynamodb.sa-east-1.amazonaws.com", Some("104.160.152.3")),
    ];

    let mut results: Vec<RiotRegionPing> = targets
        .par_iter()
        .map(|&(region, host, icmp_ip)| {
            // Try TCP 443 first (reflects actual HTTPS/TLS handshake RTT to regional PoP)
            if let Some(rtt) = tcp_probe_443(host, 1500) {
                return RiotRegionPing {
                    region: region.to_string(),
                    host: host.to_string(),
                    rtt_ms: Some(rtt),
                    method: "TCP 443".to_string(),
                };
            }
            if let Some(ip) = icmp_ip {
                if let Some(rtt) = icmp_ping_ipv4(ip, 1200) {
                    return RiotRegionPing {
                        region: region.to_string(),
                        host: ip.to_string(),
                        rtt_ms: Some(rtt),
                        method: "ICMP".to_string(),
                    };
                }
            }
            RiotRegionPing {
                region: region.to_string(),
                host: host.to_string(),
                rtt_ms: None,
                method: "Timeout".to_string(),
            }
        })
        .collect();

    results.sort_by_key(|r| r.rtt_ms.unwrap_or(u32::MAX));
    results
}

fn tcp_probe_443(host: &str, timeout_ms: u64) -> Option<u32> {
    let mut addrs = format!("{host}:443").to_socket_addrs().ok()?;
    let addr = addrs.next()?;
    let t0 = Instant::now();
    TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms)).ok()?;
    Some(t0.elapsed().as_millis() as u32)
}

#[allow(dead_code)]
fn _status_unused_guard(s: ValorantInstallStatus) -> bool {
    matches!(s, ValorantInstallStatus::Running)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valorant_ini_no_bom_patcher_tests() {
        let input_with_bom = "\u{FEFF}[/Script/ShooterGame.ShooterGameUserSettings]\r\n\
            bShouldLetterbox=False\r\n\
            bUseVSync=True\r\n\r\n\
            [ScalabilityGroups]\r\n\
            sg.ResolutionQuality=100.000000\r\n\
            sg.ShadowQuality=3\r\n";

        let out = patch_ini_content(
            input_with_bom,
            &[
                ("ScalabilityGroups", "sg.ResolutionQuality", "75.000000"),
                ("ScalabilityGroups", "sg.ShadowQuality", "0"),
                ("ScalabilityGroups", "sg.EffectsQuality", "0"),
                ("/Script/ShooterGame.ShooterGameUserSettings", "bUseVSync", "False"),
            ],
        );

        assert!(!out.starts_with('\u{FEFF}'), "BOM must be stripped!");
        assert!(out.contains("bShouldLetterbox=False"));
        assert!(out.contains("bUseVSync=False"));
        assert!(!out.contains("bUseVSync=True"));
        assert!(out.contains("sg.ResolutionQuality=75.000000"));
        assert!(out.contains("sg.ShadowQuality=0"));
        assert!(out.contains("sg.EffectsQuality=0"));
    }
}
