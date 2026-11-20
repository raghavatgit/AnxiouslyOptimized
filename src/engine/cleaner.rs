use crate::engine::maintenance::empty_recycle_bin_silent;
use crate::engine::safety::{reg_subkey_exists, service_query_state};
use crate::models::{CleanerCategory, CleanerTargetSpec, RegHive};
use crate::system::ecosystem::is_process_running_ci;
use rayon::prelude::*;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};
use winreg::RegKey;

const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;

pub fn get_default_categories(temp_min_age_hours: u64) -> Vec<CleanerCategory> {
    let _ = temp_min_age_hours;
    vec![
        CleanerCategory {
            id: "system_temp".into(),
            title: "Windows & User Temp Files".into(),
            description: "Old temporary files in %TEMP% and %SystemRoot%\\Temp (respects cooldown age window).".into(),
            is_selected: true,
            scanned_bytes: 0,
            file_count: 0,
            skipped_reason: None,
            warning_note: None,
        },
        CleanerCategory {
            id: "prefetch_cache".into(),
            title: "Windows Prefetch Cache (*.pf)".into(),
            description: "Stale application launch traces in %SystemRoot%\\Prefetch.".into(),
            is_selected: false,
            scanned_bytes: 0,
            file_count: 0,
            skipped_reason: None,
            warning_note: Some("Windows rebuilds active app prefetch traces automatically.".into()),
        },
        CleanerCategory {
            id: "gpu_shaders".into(),
            title: "DirectX, NVIDIA, AMD & Intel Shader Caches".into(),
            description: "D3DSCache, NVIDIA DXCache/GLCache/OptixCache, AMD DxCache/VkCache, Intel ShaderCache.".into(),
            is_selected: false,
            scanned_bytes: 0,
            file_count: 0,
            skipped_reason: None,
            warning_note: Some("Shaders regenerate on next game launch (brief warm-up expected once).".into()),
        },
        CleanerCategory {
            id: "thumb_cache".into(),
            title: "Windows Explorer Thumbnail Caches".into(),
            description: "Strictly targets thumbcache_*.db files in Explorer cache (preserves icon databases).".into(),
            is_selected: true,
            scanned_bytes: 0,
            file_count: 0,
            skipped_reason: None,
            warning_note: None,
        },
        CleanerCategory {
            id: "delivery_opt".into(),
            title: "Delivery Optimization P2P Cache".into(),
            description: "Cached Windows Update peer-to-peer delivery chunks.".into(),
            is_selected: true,
            scanned_bytes: 0,
            file_count: 0,
            skipped_reason: None,
            warning_note: None,
        },
        CleanerCategory {
            id: "wu_download".into(),
            title: "Windows Update Download Cache".into(),
            description: "SoftwareDistribution\\Download packages (automatically skipped if update/reboot is pending).".into(),
            is_selected: true,
            scanned_bytes: 0,
            file_count: 0,
            skipped_reason: None,
            warning_note: None,
        },
        CleanerCategory {
            id: "crash_dumps".into(),
            title: "Crash Dumps, Minidumps & WER Archives".into(),
            description: "MEMORY.DMP, Minidump (*.dmp), LiveKernelReports, CrashDumps, and Windows Error Reporting.".into(),
            is_selected: true,
            scanned_bytes: 0,
            file_count: 0,
            skipped_reason: None,
            warning_note: None,
        },
        CleanerCategory {
            id: "browser_cache".into(),
            title: "Browser HTTP, GPU & Code Caches".into(),
            description: "Chrome, Edge, Brave, Opera GX, Vivaldi, and Firefox caches across profiles (skips running browsers).".into(),
            is_selected: true,
            scanned_bytes: 0,
            file_count: 0,
            skipped_reason: None,
            warning_note: None,
        },
        CleanerCategory {
            id: "discord_spotify_cache".into(),
            title: "Discord, Spotify & Electron App Caches".into(),
            description: "Discord Cache/Code Cache/GPUCache and Spotify media storage cache (skips running apps).".into(),
            is_selected: true,
            scanned_bytes: 0,
            file_count: 0,
            skipped_reason: None,
            warning_note: None,
        },
        CleanerCategory {
            id: "launcher_webcache".into(),
            title: "Steam, Epic Games & Riot Client Caches".into(),
            description: "Steam htmlcache/httpcache, Epic Games webcache, and Riot Client log caches.".into(),
            is_selected: true,
            scanned_bytes: 0,
            file_count: 0,
            skipped_reason: None,
            warning_note: None,
        },
        CleanerCategory {
            id: "windows_servicing_logs".into(),
            title: "Windows CBS, DISM & Panther Servicing Logs".into(),
            description: "Old servicing logs in C:\\Windows\\Logs\\CBS, DISM, and Panther (*.log).".into(),
            is_selected: true,
            scanned_bytes: 0,
            file_count: 0,
            skipped_reason: None,
            warning_note: None,
        },
        CleanerCategory {
            id: "font_cache".into(),
            title: "Windows Font & Presentation Cache".into(),
            description: "Stale FontCache*.dat files in LocalService FontCache (rebuilt automatically).".into(),
            is_selected: false,
            scanned_bytes: 0,
            file_count: 0,
            skipped_reason: None,
            warning_note: None,
        },
        CleanerCategory {
            id: "recycle_bin".into(),
            title: "Windows Recycle Bin (All Drives)".into(),
            description: "Deleted items sitting in $Recycle.Bin across fixed volumes.".into(),
            is_selected: true,
            scanned_bytes: 0,
            file_count: 0,
            skipped_reason: None,
            warning_note: None,
        },
        CleanerCategory {
            id: "game_emu_logs".into(),
            title: "Game & Android Emulator Logs".into(),
            description: "Logs and crash traces from VALORANT, BlueStacks 5, MSI App Player, and LDPlayer 9.".into(),
            is_selected: true,
            scanned_bytes: 0,
            file_count: 0,
            skipped_reason: None,
            warning_note: None,
        },
    ]
}

fn category_specs(id: &str, temp_min_age_hours: u64) -> (Vec<CleanerTargetSpec>, Option<String>) {
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    let local_low = std::env::var("USERPROFILE")
        .map(|p| format!(r"{p}\AppData\LocalLow"))
        .unwrap_or_default();
    let temp = std::env::var("TEMP").unwrap_or_default();
    let windir = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    let progdata = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".to_string());
    let pfx86 = std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| r"C:\Program Files (x86)".to_string());

    let min_age = if temp_min_age_hours > 0 {
        Some(temp_min_age_hours)
    } else {
        None
    };

    match id {
        "system_temp" => (
            vec![
                CleanerTargetSpec {
                    base_path: temp,
                    wildcard: None,
                    min_age_hours: min_age,
                },
                CleanerTargetSpec {
                    base_path: format!(r"{windir}\Temp"),
                    wildcard: None,
                    min_age_hours: min_age,
                },
            ],
            None,
        ),
        "prefetch_cache" => (
            vec![CleanerTargetSpec {
                base_path: format!(r"{windir}\Prefetch"),
                wildcard: Some("*.pf".to_string()),
                min_age_hours: None,
            }],
            None,
        ),
        "gpu_shaders" => (
            vec![
                CleanerTargetSpec { base_path: format!(r"{local}\D3DSCache"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{local}\NVIDIA\DXCache"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{local}\NVIDIA\GLCache"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{local}\NVIDIA\OptixCache"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{local_low}\NVIDIA\PerDriverVersion\DXCache"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{local_low}\NVIDIA\PerDriverVersion\GLCache"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{local}\AMD\DxCache"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{local}\AMD\DxcCache"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{local}\AMD\VkCache"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{local}\AMD\OglCache"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{local_low}\Intel\ShaderCache"), wildcard: None, min_age_hours: None },
            ],
            None,
        ),
        "thumb_cache" => (
            vec![CleanerTargetSpec {
                base_path: format!(r"{local}\Microsoft\Windows\Explorer"),
                wildcard: Some("thumbcache_*.db".to_string()),
                min_age_hours: None,
            }],
            None,
        ),
        "delivery_opt" => (
            vec![
                CleanerTargetSpec {
                    base_path: format!(
                        r"{windir}\ServiceProfiles\NetworkService\AppData\Local\Microsoft\Windows\DeliveryOptimization\Cache"
                    ),
                    wildcard: None,
                    min_age_hours: None,
                },
                CleanerTargetSpec {
                    base_path: format!(r"{windir}\SoftwareDistribution\DeliveryOptimization"),
                    wildcard: None,
                    min_age_hours: None,
                },
            ],
            None,
        ),
        "wu_download" => {
            if is_windows_update_pending() {
                (
                    Vec::new(),
                    Some("Skipped: Windows Update or reboot is currently pending".to_string()),
                )
            } else {
                (
                    vec![CleanerTargetSpec {
                        base_path: format!(r"{windir}\SoftwareDistribution\Download"),
                        wildcard: None,
                        min_age_hours: None,
                    }],
                    None,
                )
            }
        }
        "crash_dumps" => (
            vec![
                CleanerTargetSpec { base_path: format!(r"{windir}\MEMORY.DMP"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{windir}\Minidump"), wildcard: Some("*.dmp".to_string()), min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{windir}\LiveKernelReports"), wildcard: Some("*.dmp".to_string()), min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{local}\CrashDumps"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{progdata}\Microsoft\Windows\WER\ReportArchive"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{progdata}\Microsoft\Windows\WER\ReportQueue"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{local}\Microsoft\Windows\WER\ReportArchive"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{local}\Microsoft\Windows\WER\ReportQueue"), wildcard: None, min_age_hours: None },
            ],
            None,
        ),
        "browser_cache" => {
            let mut specs = Vec::new();
            let mut skipped_browsers = Vec::new();

            if is_process_running_ci("chrome") {
                skipped_browsers.push("Chrome (running)");
            } else {
                append_chromium_profile_caches(&format!(r"{local}\Google\Chrome\User Data"), &mut specs);
            }
            if is_process_running_ci("msedge") {
                skipped_browsers.push("Edge (running)");
            } else {
                append_chromium_profile_caches(&format!(r"{local}\Microsoft\Edge\User Data"), &mut specs);
            }
            if is_process_running_ci("brave") {
                skipped_browsers.push("Brave (running)");
            } else {
                append_chromium_profile_caches(&format!(r"{local}\BraveSoftware\Brave-Browser\User Data"), &mut specs);
            }
            if is_process_running_ci("vivaldi") {
                skipped_browsers.push("Vivaldi (running)");
            } else {
                append_chromium_profile_caches(&format!(r"{local}\Vivaldi\User Data"), &mut specs);
            }
            if is_process_running_ci("opera") {
                skipped_browsers.push("Opera (running)");
            } else {
                for sub in ["Cache", "Code Cache", "GPUCache"] {
                    specs.push(CleanerTargetSpec {
                        base_path: format!(r"{local}\Opera Software\Opera GX Stable\{sub}"),
                        wildcard: None,
                        min_age_hours: None,
                    });
                    specs.push(CleanerTargetSpec {
                        base_path: format!(r"{local}\Opera Software\Opera Stable\{sub}"),
                        wildcard: None,
                        min_age_hours: None,
                    });
                }
            }
            if is_process_running_ci("firefox") {
                skipped_browsers.push("Firefox (running)");
            } else {
                append_firefox_caches(&format!(r"{local}\Mozilla\Firefox\Profiles"), &mut specs);
            }

            let skip_msg = if skipped_browsers.is_empty() {
                None
            } else {
                Some(format!("Skipped active: {}", skipped_browsers.join(", ")))
            };
            (specs, skip_msg)
        }
        "discord_spotify_cache" => {
            let mut specs = Vec::new();
            let mut skipped = Vec::new();
            if is_process_running_ci("discord") {
                skipped.push("Discord (running)");
            } else {
                for sub in ["Cache", "Code Cache", "GPUCache", "DawnCache"] {
                    specs.push(CleanerTargetSpec {
                        base_path: format!(r"{appdata}\discord\{sub}"),
                        wildcard: None,
                        min_age_hours: None,
                    });
                }
            }
            if is_process_running_ci("spotify") {
                skipped.push("Spotify (running)");
            } else {
                specs.push(CleanerTargetSpec {
                    base_path: format!(r"{local}\Spotify\Storage"),
                    wildcard: None,
                    min_age_hours: None,
                });
                specs.push(CleanerTargetSpec {
                    base_path: format!(r"{local}\Spotify\Browser\Cache"),
                    wildcard: None,
                    min_age_hours: None,
                });
            }
            let skip_msg = if skipped.is_empty() {
                None
            } else {
                Some(format!("Skipped active: {}", skipped.join(", ")))
            };
            (specs, skip_msg)
        }
        "launcher_webcache" => (
            vec![
                CleanerTargetSpec { base_path: format!(r"{local}\Steam\htmlcache\Cache"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{local}\Steam\htmlcache\Code Cache"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{local}\Steam\htmlcache\GPUCache"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{pfx86}\Steam\appcache\httpcache"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{local}\EpicGamesLauncher\Saved\Logs"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{local}\Riot Games\Riot Client\Logs"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{local}\Riot Games\Riot Client\Crashes"), wildcard: None, min_age_hours: None },
            ],
            None,
        ),
        "windows_servicing_logs" => (
            vec![
                CleanerTargetSpec { base_path: format!(r"{windir}\Logs\CBS"), wildcard: Some("*.log".to_string()), min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{windir}\Logs\DISM"), wildcard: Some("*.log".to_string()), min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{windir}\Panther"), wildcard: Some("*.log".to_string()), min_age_hours: None },
            ],
            None,
        ),
        "font_cache" => (
            vec![CleanerTargetSpec {
                base_path: format!(r"{windir}\ServiceProfiles\LocalService\AppData\Local\FontCache"),
                wildcard: Some("*FontCache*.dat".to_string()),
                min_age_hours: None,
            }],
            None,
        ),
        "recycle_bin" => {
            let mut specs = Vec::new();
            for letter in ['C', 'D', 'E', 'F', 'G'] {
                let p = format!(r"{letter}:\$Recycle.Bin");
                if Path::new(&p).exists() {
                    specs.push(CleanerTargetSpec {
                        base_path: p,
                        wildcard: None,
                        min_age_hours: None,
                    });
                }
            }
            (specs, None)
        }
        "game_emu_logs" => (
            vec![
                CleanerTargetSpec { base_path: format!(r"{local}\VALORANT\Saved\Logs"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{local}\VALORANT\Saved\Crashes"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{local}\BlueStacks_nxt\Logs"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{progdata}\BlueStacks_nxt\Logs"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{progdata}\BlueStacks_msi5\Logs"), wildcard: None, min_age_hours: None },
                CleanerTargetSpec { base_path: format!(r"{local}\LDPlayer9\Log"), wildcard: None, min_age_hours: None },
            ],
            None,
        ),
        _ => (Vec::new(), None),
    }
}

fn append_chromium_profile_caches(user_data_dir: &str, specs: &mut Vec<CleanerTargetSpec>) {
    let root = Path::new(user_data_dir);
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for ent in entries.flatten() {
        let p = ent.path();
        if !p.is_dir() || is_reparse_point(&p) {
            continue;
        }
        let name = ent.file_name().to_string_lossy().to_string();
        if name.eq_ignore_ascii_case("Default") || name.starts_with("Profile ") {
            for sub in ["Cache", "Code Cache", "GPUCache", "DawnCache"] {
                specs.push(CleanerTargetSpec {
                    base_path: p.join(sub).to_string_lossy().to_string(),
                    wildcard: None,
                    min_age_hours: None,
                });
            }
        }
    }
}

fn append_firefox_caches(profiles_dir: &str, specs: &mut Vec<CleanerTargetSpec>) {
    let Ok(entries) = std::fs::read_dir(profiles_dir) else {
        return;
    };
    for ent in entries.flatten() {
        let p = ent.path();
        if p.is_dir() && !is_reparse_point(&p) {
            specs.push(CleanerTargetSpec {
                base_path: p.join("cache2").to_string_lossy().to_string(),
                wildcard: None,
                min_age_hours: None,
            });
            specs.push(CleanerTargetSpec {
                base_path: p.join("startupCache").to_string_lossy().to_string(),
                wildcard: None,
                min_age_hours: None,
            });
        }
    }
}

fn is_windows_update_pending() -> bool {
    if reg_subkey_exists(
        RegHive::Hklm,
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Component Based Servicing\RebootPending",
    ) || reg_subkey_exists(
        RegHive::Hklm,
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\WindowsUpdate\Auto Update\RebootRequired",
    ) {
        return true;
    }

    if let Ok((_, state)) = service_query_state("wuauserv") {
        if state == "Running" {
            let has_staging = RegKey::predef(HKEY_LOCAL_MACHINE)
                .open_subkey_with_flags(
                    r"SOFTWARE\Microsoft\Windows\CurrentVersion\WindowsUpdate\Auto Update",
                    KEY_READ | KEY_WOW64_64KEY,
                )
                .and_then(|k| k.get_value::<u32, _>("AUState"))
                .map(|v| v >= 3)
                .unwrap_or(false);
            if has_staging {
                return true;
            }
        }
    }
    false
}

pub fn scan_categories(categories: &mut [CleanerCategory], temp_min_age_hours: u64) {
    categories.par_iter_mut().for_each(|cat| {
        let (specs, skip_reason) = category_specs(&cat.id, temp_min_age_hours);
        cat.skipped_reason = skip_reason;
        let files = collect_matching_files(&specs);
        let total_bytes: u64 = files.iter().map(|(_, sz)| *sz).sum();
        cat.scanned_bytes = total_bytes;
        cat.file_count = files.len();
    });
}

pub fn purge_selected_categories(
    categories: &mut [CleanerCategory],
    temp_min_age_hours: u64,
) -> (u64, usize, usize, Vec<String>) {
    let mut freed_bytes = 0u64;
    let mut deleted_files = 0usize;
    let mut failed_files = 0usize;
    let mut logs = Vec::new();

    for cat in categories.iter_mut() {
        if !cat.is_selected {
            continue;
        }
        let (specs, skip_reason) = category_specs(&cat.id, temp_min_age_hours);
        cat.skipped_reason = skip_reason.clone();
        if let Some(reason) = skip_reason {
            logs.push(format!("[SKIP] {}: {}", cat.title, reason));
            if specs.is_empty() {
                continue;
            }
        }

        let (mut cat_freed, mut cat_del, cat_fail) = purge_specs_safe(&specs);

        if cat.id == "recycle_bin" {
            let before_bytes = cat.scanned_bytes;
            let before_cnt = cat.file_count;
            let _ = empty_recycle_bin_silent();
            if cat_freed < before_bytes {
                cat_freed = before_bytes;
            }
            if cat_del < before_cnt {
                cat_del = before_cnt;
            }
        }

        freed_bytes += cat_freed;
        deleted_files += cat_del;
        failed_files += cat_fail;

        let remaining = collect_matching_files(&specs);
        cat.scanned_bytes = remaining.iter().map(|(_, s)| *s).sum();
        cat.file_count = remaining.len();

        logs.push(format!(
            "[CLEAN] {}: deleted {} file(s), reclaimed {} ({} locked/skipped)",
            cat.title,
            cat_del,
            crate::engine::format_bytes(cat_freed),
            cat_fail
        ));
    }

    (freed_bytes, deleted_files, failed_files, logs)
}

/// Safely deletes files matching `specs` (skipping reparse points/junctions) and prunes empty folders.
pub fn purge_specs_safe(specs: &[CleanerTargetSpec]) -> (u64, usize, usize) {
    let files = collect_matching_files(specs);
    let mut freed = 0u64;
    let mut deleted = 0usize;
    let mut failed = 0usize;

    for (path, size) in files {
        match std::fs::remove_file(&path) {
            Ok(()) => {
                freed += size;
                deleted += 1;
            }
            Err(_) => {
                failed += 1;
            }
        }
    }

    for spec in specs {
        if spec.wildcard.is_none() {
            let p = Path::new(&spec.base_path);
            if p.is_dir() && !is_reparse_point(p) {
                remove_empty_subfolders(p);
            }
        }
    }

    (freed, deleted, failed)
}

pub fn collect_matching_files(specs: &[CleanerTargetSpec]) -> Vec<(PathBuf, u64)> {
    let mut out = Vec::new();
    let now = SystemTime::now();
    for spec in specs {
        if spec.base_path.is_empty() {
            continue;
        }
        let root = Path::new(&spec.base_path);
        if !root.exists() || is_reparse_point(root) {
            continue;
        }
        if root.is_file() {
            if let Ok(meta) = std::fs::symlink_metadata(root) {
                if age_allows(&meta, spec.min_age_hours, now) {
                    out.push((root.to_path_buf(), meta.len()));
                }
            }
            continue;
        }
        walk_dir_safe(
            root,
            spec.wildcard.as_deref(),
            spec.min_age_hours,
            now,
            &mut out,
            0,
        );
    }
    out
}

fn walk_dir_safe(
    dir: &Path,
    wildcard: Option<&str>,
    min_age_hours: Option<u64>,
    now: SystemTime,
    out: &mut Vec<(PathBuf, u64)>,
    depth: usize,
) {
    if depth > 24 || is_reparse_point(dir) {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for ent in entries.flatten() {
        let p = ent.path();
        if is_reparse_point(&p) {
            continue;
        }
        let Ok(meta) = std::fs::symlink_metadata(&p) else {
            continue;
        };
        if meta.is_dir() {
            if wildcard != Some("thumbcache_*.db") {
                walk_dir_safe(&p, wildcard, min_age_hours, now, out, depth + 1);
            }
        } else if meta.is_file() {
            if !age_allows(&meta, min_age_hours, now) {
                continue;
            }
            if let Some(pat) = wildcard {
                let fname = p
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                if !matches_wildcard(&fname, pat) {
                    continue;
                }
            }
            out.push((p, meta.len()));
        }
    }
}

fn remove_empty_subfolders(root: &Path) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for ent in entries.flatten() {
        let p = ent.path();
        if is_reparse_point(&p) {
            continue;
        }
        if p.is_dir() {
            remove_empty_subfolders_inner(&p, 0);
        }
    }
}

fn remove_empty_subfolders_inner(dir: &Path, depth: usize) {
    if depth > 20 || is_reparse_point(dir) {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for ent in entries.flatten() {
            let p = ent.path();
            if p.is_dir() && !is_reparse_point(&p) {
                remove_empty_subfolders_inner(&p, depth + 1);
            }
        }
    }
    let _ = std::fs::remove_dir(dir);
}

fn age_allows(meta: &std::fs::Metadata, min_age_hours: Option<u64>, now: SystemTime) -> bool {
    let Some(hours) = min_age_hours else {
        return true;
    };
    if hours == 0 {
        return true;
    }
    let min_dur = Duration::from_secs(hours.saturating_mul(3600));
    match meta.modified() {
        Ok(mtime) => now.duration_since(mtime).map(|d| d >= min_dur).unwrap_or(false),
        Err(_) => true,
    }
}

pub fn is_reparse_point(path: &Path) -> bool {
    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            meta.file_type().is_symlink() || (meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT) != 0
        }
        Err(_) => false,
    }
}

pub fn matches_wildcard(text: &str, pattern: &str) -> bool {
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    wildcard_match_slice(&t, &p)
}

fn wildcard_match_slice(t: &[char], p: &[char]) -> bool {
    let (mut ti, mut pi) = (0usize, 0usize);
    let mut star_pi: Option<usize> = None;
    let mut match_ti = 0usize;

    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            ti += 1;
            pi += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star_pi = Some(pi);
            match_ti = ti;
            pi += 1;
        } else if let Some(sp) = star_pi {
            pi = sp + 1;
            match_ti += 1;
            ti = match_ti;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleaner_wildcard_filter_tests() {
        assert!(matches_wildcard("thumbcache_256.db", "thumbcache_*.db"));
        assert!(matches_wildcard("THUMBCACHE_96.DB", "thumbcache_*.db"));
        assert!(!matches_wildcard("iconcache_32.db", "thumbcache_*.db"));
        assert!(!matches_wildcard("ExplorerStartupLog.etl", "thumbcache_*.db"));

        let dir = std::env::temp_dir().join("obr_wildcard_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("thumbcache_idx.db"), b"12345").unwrap();
        std::fs::write(dir.join("iconcache_16.db"), b"123456").unwrap();

        let specs = vec![CleanerTargetSpec {
            base_path: dir.to_string_lossy().to_string(),
            wildcard: Some("thumbcache_*.db".into()),
            min_age_hours: None,
        }];
        let matched = collect_matching_files(&specs);
        assert_eq!(matched.len(), 1);
        assert!(matched[0]
            .0
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("thumbcache_"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
