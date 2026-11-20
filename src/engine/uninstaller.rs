use crate::engine::cleaner::is_reparse_point;
use crate::engine::debloat::is_protected_package;
use crate::engine::safety::open_hive_root;
use crate::engine::{run_powershell_encoded, CREATE_NO_WINDOW};
use crate::models::{InstalledAppEntry, InstalledAppKind, LeftoverItem, LeftoverKind, RegHive};
use std::collections::HashSet;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use winreg::enums::{
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY, KEY_WRITE,
};
use winreg::RegKey;

const UNINSTALL_SUBKEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";

/// Protected vendor/system token stems that Leftover Scanner MUST NEVER match as generic directory/registry names.
const GENERIC_PROTECTED_TOKENS: &[&str] = &[
    "windows",
    "microsoft",
    "common files",
    "internet explorer",
    "windows nt",
    "windows defender",
    "windows photo viewer",
    "windows mail",
    "windows media player",
    "WindowsApps",
    "package cache",
    "system",
    "drivers",
    "policies",
    "classes",
    "clients",
    "currentversion",
    "default",
    "google",
    "valve",
    "riot games",
];

pub fn scan_all_installed_apps(include_uwp: bool) -> Vec<InstalledAppEntry> {
    let mut apps = Vec::new();
    let mut seen_names = HashSet::new();

    // 1. Win32 64-bit HKLM
    scan_registry_uninstall_branch(
        RegHive::Hklm,
        KEY_READ | KEY_WOW64_64KEY,
        true,
        InstalledAppKind::Win32x64,
        &mut apps,
        &mut seen_names,
    );

    // 2. Win32 32-bit WOW6432Node HKLM
    scan_registry_uninstall_branch(
        RegHive::Hklm,
        KEY_READ | KEY_WOW64_32KEY,
        false,
        InstalledAppKind::Win32x86,
        &mut apps,
        &mut seen_names,
    );

    // 3. Per-user HKCU
    scan_registry_uninstall_branch(
        RegHive::Hkcu,
        KEY_READ | KEY_WOW64_64KEY,
        true,
        InstalledAppKind::UserHkcu,
        &mut apps,
        &mut seen_names,
    );

    // 4. Optional Windows Store / UWP non-framework packages
    if include_uwp {
        if let Ok(uwp) = scan_uwp_apps() {
            apps.extend(uwp);
        }
    }

    apps.sort_by(|a, b| {
        a.display_name
            .to_ascii_lowercase()
            .cmp(&b.display_name.to_ascii_lowercase())
    });
    apps
}

fn scan_registry_uninstall_branch(
    hive: RegHive,
    flags: u32,
    is_win64_hive: bool,
    kind: InstalledAppKind,
    out: &mut Vec<InstalledAppEntry>,
    seen: &mut HashSet<String>,
) {
    let root = match hive {
        RegHive::Hklm => RegKey::predef(HKEY_LOCAL_MACHINE),
        RegHive::Hkcu => RegKey::predef(HKEY_CURRENT_USER),
    };
    let Ok(uninst) = root.open_subkey_with_flags(UNINSTALL_SUBKEY, flags) else {
        return;
    };

    for key_name in uninst.enum_keys().flatten() {
        let Ok(sub) = uninst.open_subkey_with_flags(&key_name, flags) else {
            continue;
        };

        // Skip Windows system components & update patches
        if sub.get_value::<u32, _>("SystemComponent").unwrap_or(0) == 1 {
            continue;
        }
        let release_type: String = sub.get_value("ReleaseType").unwrap_or_default();
        if release_type.eq_ignore_ascii_case("Security Update")
            || release_type.eq_ignore_ascii_case("Update")
            || release_type.eq_ignore_ascii_case("Hotfix")
        {
            continue;
        }
        let parent_key: String = sub.get_value("ParentKeyName").unwrap_or_default();
        if !parent_key.is_empty() {
            continue;
        }

        let display_name: String = sub
            .get_value::<String, _>("DisplayName")
            .unwrap_or_default()
            .trim()
            .to_string();
        if display_name.is_empty() {
            continue;
        }

        let uninstall_string: Option<String> = sub
            .get_value::<String, _>("UninstallString")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let quiet_uninstall_string: Option<String> = sub
            .get_value::<String, _>("QuietUninstallString")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        if uninstall_string.is_none() && quiet_uninstall_string.is_none() {
            continue;
        }

        let display_version: String = sub
            .get_value::<String, _>("DisplayVersion")
            .unwrap_or_default()
            .trim()
            .to_string();
        let publisher: String = sub
            .get_value::<String, _>("Publisher")
            .unwrap_or_default()
            .trim()
            .to_string();
        let raw_date: String = sub
            .get_value::<String, _>("InstallDate")
            .unwrap_or_default()
            .trim()
            .to_string();
        let install_date = format_yyyymmdd(&raw_date);

        let install_location: Option<String> = sub
            .get_value::<String, _>("InstallLocation")
            .ok()
            .map(|s| s.trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty());

        let est_kb: u32 = sub.get_value("EstimatedSize").unwrap_or(0);
        let mut estimated_size_bytes = (est_kb as u64).saturating_mul(1024);
        if estimated_size_bytes == 0 {
            if let Some(ref loc) = install_location {
                let p = Path::new(loc);
                if is_safe_specific_folder(p) {
                    estimated_size_bytes = shallow_folder_size(p, 2);
                }
            }
        }

        let dedup_key = format!(
            "{}|{}",
            display_name.to_ascii_lowercase(),
            display_version.to_ascii_lowercase()
        );
        if !seen.insert(dedup_key) {
            continue;
        }

        let full_subkey = format!(r"{UNINSTALL_SUBKEY}\{key_name}");
        let id = format!(
            "{}:{}:{}",
            hive.cli_prefix(),
            if is_win64_hive { "64" } else { "32" },
            key_name
        );

        out.push(InstalledAppEntry {
            id,
            display_name,
            display_version,
            publisher,
            install_date,
            estimated_size_bytes,
            install_location,
            uninstall_string,
            quiet_uninstall_string,
            registry_hive: Some(hive),
            registry_subkey: full_subkey,
            is_win64_hive,
            kind,
        });
    }
}

fn scan_uwp_apps() -> Result<Vec<InstalledAppEntry>, String> {
    let script = "Get-AppxPackage | Where-Object { -not $_.IsFramework -and -not $_.NonRemovable -and $_.SignatureKind -ne 'System' } | Select-Object Name, Version, Publisher, PackageFullName, InstallLocation | ForEach-Object { \"$($_.Name)|$($_.Version)|$($_.Publisher)|$($_.PackageFullName)|$($_.InstallLocation)\" }";
    let out = run_powershell_encoded(script, 25_000)?;
    if out.exit_code != 0 {
        return Err("Get-AppxPackage query failed".to_string());
    }

    let mut list = Vec::new();
    for line in out.stdout.lines() {
        let parts: Vec<&str> = line.trim().split('|').collect();
        if parts.len() < 4 {
            continue;
        }
        let name = parts[0].trim();
        if name.is_empty() || is_protected_package(name) {
            continue;
        }
        let ver = parts[1].trim().to_string();
        let pub_raw = parts[2].trim().trim_start_matches("CN=").to_string();
        let full_name = parts[3].trim().to_string();
        let loc = parts
            .get(4)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        list.push(InstalledAppEntry {
            id: format!("UWP:{full_name}"),
            display_name: name.to_string(),
            display_version: ver,
            publisher: pub_raw,
            install_date: String::new(),
            estimated_size_bytes: 0,
            install_location: loc,
            uninstall_string: Some(full_name.clone()),
            quiet_uninstall_string: None,
            registry_hive: None,
            registry_subkey: full_name,
            is_win64_hive: true,
            kind: InstalledAppKind::StoreUwp,
        });
    }
    Ok(list)
}

fn format_yyyymmdd(s: &str) -> String {
    if s.len() == 8 && s.chars().all(|c| c.is_ascii_digit()) {
        format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8])
    } else {
        s.to_string()
    }
}

/// Executes the program's standard uninstaller (or `Remove-AppxPackage` for UWP) and waits for completion,
/// ready for Geek.exe automatic leftover scan afterward.
pub fn run_standard_uninstall(app: &InstalledAppEntry) -> Result<String, String> {
    if app.kind == InstalledAppKind::StoreUwp {
        let escaped = app.registry_subkey.replace('\'', "''");
        let script = format!(
            "Remove-AppxPackage -Package '{}' -ErrorAction Stop",
            escaped
        );
        let out = run_powershell_encoded(&script, 60_000)?;
        if out.exit_code == 0 {
            return Ok(format!(
                "Uninstalled Store package '{}'. Scanning for leftovers...",
                app.display_name
            ));
        } else {
            return Err(format!(
                "Remove-AppxPackage failed: {}",
                out.stderr.trim()
            ));
        }
    }

    let cmd_line = app
        .uninstall_string
        .as_deref()
        .or(app.quiet_uninstall_string.as_deref())
        .ok_or_else(|| "No UninstallString registered for this entry".to_string())?;

    // Launch via cmd.exe /C so quoted paths + arguments + msiexec /I{GUID} work seamlessly
    let status = Command::new("cmd.exe")
        .args(["/C", cmd_line])
        .status()
        .map_err(|e| format!("Failed launching uninstaller '{cmd_line}': {e}"))?;

    Ok(format!(
        "Uninstaller for '{}' finished (exit {}). Scanning leftovers...",
        app.display_name,
        status.code().unwrap_or(0)
    ))
}

/// Geek.exe Deep Leftover Scanner:
/// Finds remaining folders, files, shortcuts, and registry keys/values belonging to `app`.
pub fn scan_leftovers_for_app(app: &InstalledAppEntry) -> Vec<LeftoverItem> {
    let mut items = Vec::new();
    let mut seen_paths = HashSet::new();

    // 1. Check if the Uninstall Registry key itself still exists (crucial for Force Removal & broken uninstallers!)
    if let Some(hive) = app.registry_hive {
        let flags = KEY_READ
            | if app.is_win64_hive {
                KEY_WOW64_64KEY
            } else {
                KEY_WOW64_32KEY
            };
        if open_hive_root(hive)
            .open_subkey_with_flags(&app.registry_subkey, flags)
            .is_ok()
        {
            let disp = format!("{}\\{}", hive.cli_prefix(), app.registry_subkey);
            if seen_paths.insert(disp.to_ascii_lowercase()) {
                items.push(LeftoverItem {
                    kind: LeftoverKind::RegistryKey,
                    display_path: format!("{disp} [Uninstall Registration]"),
                    fs_path: None,
                    hive: Some(hive),
                    subkey: app.registry_subkey.clone(),
                    value_name: None,
                    is_win64_hive: app.is_win64_hive,
                    size_bytes: 0,
                    is_selected: true,
                });
            }
        }
    }

    // 2. Check explicit InstallLocation if safe & specific
    if let Some(ref loc) = app.install_location {
        let p = Path::new(loc);
        if p.exists() && is_safe_specific_folder(p) && !is_reparse_point(p) {
            let disp = p.to_string_lossy().to_string();
            if seen_paths.insert(disp.to_ascii_lowercase()) {
                let sz = shallow_folder_size(p, 4);
                items.push(LeftoverItem {
                    kind: LeftoverKind::Directory,
                    display_path: disp.clone(),
                    fs_path: Some(disp),
                    hive: None,
                    subkey: String::new(),
                    value_name: None,
                    is_win64_hive: true,
                    size_bytes: sz,
                    is_selected: true,
                });
            }
        }
    }

    // Build safe search tokens from display_name (minimum 4 chars, stripped of version/arch noise, not in GENERIC_PROTECTED_TOKENS)
    let tokens = extract_safe_app_tokens(&app.display_name, &app.publisher);
    if tokens.is_empty() {
        return items;
    }

    // 3. Scan standard AppData / ProgramData / Program Files / Start Menu directories for exact folder/shortcut matches
    let mut scan_roots = Vec::new();
    for env_key in [
        "LOCALAPPDATA",
        "APPDATA",
        "ProgramData",
        "ProgramFiles",
        "ProgramFiles(x86)",
    ] {
        if let Ok(v) = std::env::var(env_key) {
            if !v.is_empty() {
                scan_roots.push(PathBuf::from(&v));
            }
        }
    }
    if let Ok(ap) = std::env::var("APPDATA") {
        scan_roots.push(PathBuf::from(ap).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Ok(pd) = std::env::var("ProgramData") {
        scan_roots.push(PathBuf::from(pd).join(r"Microsoft\Windows\Start Menu\Programs"));
    }

    for root in scan_roots {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for ent in entries.flatten() {
            let p = ent.path();
            if is_reparse_point(&p) {
                continue;
            }
            let fname = ent.file_name().to_string_lossy().to_string();
            if matches_app_token_strictly(&fname, &tokens) && is_safe_specific_folder(&p) {
                let disp = p.to_string_lossy().to_string();
                if seen_paths.insert(disp.to_ascii_lowercase()) {
                    let is_dir = p.is_dir();
                    let sz = if is_dir {
                        shallow_folder_size(&p, 3)
                    } else {
                        std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0)
                    };
                    items.push(LeftoverItem {
                        kind: if is_dir {
                            LeftoverKind::Directory
                        } else {
                            LeftoverKind::File
                        },
                        display_path: disp.clone(),
                        fs_path: Some(disp),
                        hive: None,
                        subkey: String::new(),
                        value_name: None,
                        is_win64_hive: true,
                        size_bytes: sz,
                        is_selected: true,
                    });
                }
            }
        }
    }

    // 4. Scan HKCU\Software and HKLM\SOFTWARE top-level keys for exact token match
    for (hive, base_sub, is_w64) in [
        (RegHive::Hkcu, "Software", true),
        (RegHive::Hklm, "SOFTWARE", true),
        (RegHive::Hklm, "SOFTWARE", false),
    ] {
        let flags = KEY_READ | if is_w64 { KEY_WOW64_64KEY } else { KEY_WOW64_32KEY };
        let root = open_hive_root(hive);
        if let Ok(sw) = root.open_subkey_with_flags(base_sub, flags) {
            for kname in sw.enum_keys().flatten() {
                if matches_app_token_strictly(&kname, &tokens) {
                    let sub = format!(r"{base_sub}\{kname}");
                    let disp = format!(
                        "{}{}\\{sub}",
                        hive.cli_prefix(),
                        if is_w64 { "" } else { " (WOW6432)" }
                    );
                    if seen_paths.insert(disp.to_ascii_lowercase()) {
                        items.push(LeftoverItem {
                            kind: LeftoverKind::RegistryKey,
                            display_path: disp,
                            fs_path: None,
                            hive: Some(hive),
                            subkey: sub,
                            value_name: None,
                            is_win64_hive: is_w64,
                            size_bytes: 0,
                            is_selected: true,
                        });
                    }
                }
            }
        }
    }

    // 5. Scan AppCompatFlags\Layers & DirectX\UserGpuPreferences for orphaned EXE references
    for sub in [
        r"Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers",
        r"Software\Microsoft\DirectX\UserGpuPreferences",
    ] {
        if let Ok(k) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(sub, KEY_READ) {
            for (vname, _) in k.enum_values().flatten() {
                if matches_app_token_strictly(&vname, &tokens) {
                    let disp = format!("HKCU\\{sub} -> {vname}");
                    if seen_paths.insert(disp.to_ascii_lowercase()) {
                        items.push(LeftoverItem {
                            kind: LeftoverKind::RegistryValue,
                            display_path: disp,
                            fs_path: None,
                            hive: Some(RegHive::Hkcu),
                            subkey: sub.to_string(),
                            value_name: Some(vname),
                            is_win64_hive: true,
                            size_bytes: 0,
                            is_selected: true,
                        });
                    }
                }
            }
        }
    }

    items
}

/// Removes all selected `LeftoverItem`s (files, directories, registry keys, registry values).
pub fn remove_selected_leftovers(items: &[LeftoverItem]) -> (usize, usize, u64) {
    let mut deleted = 0usize;
    let mut failed = 0usize;
    let mut freed_bytes = 0u64;

    for item in items.iter().filter(|i| i.is_selected) {
        match item.kind {
            LeftoverKind::File => {
                if let Some(ref p_str) = item.fs_path {
                    let p = Path::new(p_str);
                    if !p.exists() || (!is_reparse_point(p) && std::fs::remove_file(p).is_ok()) {
                        deleted += 1;
                        freed_bytes += item.size_bytes;
                    } else {
                        failed += 1;
                    }
                }
            }
            LeftoverKind::Directory => {
                if let Some(ref p_str) = item.fs_path {
                    let p = Path::new(p_str);
                    if !p.exists() {
                        deleted += 1;
                    } else if is_safe_specific_folder(p)
                        && !is_reparse_point(p)
                        && std::fs::remove_dir_all(p).is_ok()
                    {
                        deleted += 1;
                        freed_bytes += item.size_bytes;
                    } else {
                        failed += 1;
                    }
                }
            }
            LeftoverKind::RegistryKey => {
                if let Some(hive) = item.hive {
                    let flags = KEY_READ
                        | KEY_WRITE
                        | if item.is_win64_hive {
                            KEY_WOW64_64KEY
                        } else {
                            KEY_WOW64_32KEY
                        };
                    let root = open_hive_root(hive);
                    if let Some((parent, child)) = item.subkey.rsplit_once('\\') {
                        if let Ok(pk) = root.open_subkey_with_flags(parent, flags) {
                            if pk.delete_subkey_all(child).is_ok() {
                                deleted += 1;
                                continue;
                            }
                        }
                    }
                    failed += 1;
                }
            }
            LeftoverKind::RegistryValue => {
                if let (Some(hive), Some(ref val_name)) = (item.hive, &item.value_name) {
                    let flags = KEY_WRITE
                        | if item.is_win64_hive {
                            KEY_WOW64_64KEY
                        } else {
                            KEY_WOW64_32KEY
                        };
                    if let Ok(k) = open_hive_root(hive).open_subkey_with_flags(&item.subkey, flags) {
                        if k.delete_value(val_name).is_ok() {
                            deleted += 1;
                            continue;
                        }
                    }
                    failed += 1;
                }
            }
        }
    }

    (deleted, failed, freed_bytes)
}

/// Opens Windows Registry Editor (`regedit.exe`) jumped directly to `hive\subkey`.
pub fn open_in_regedit(hive: RegHive, subkey: &str) -> Result<(), String> {
    let full_key = match hive {
        RegHive::Hklm => format!(r"Computer\HKEY_LOCAL_MACHINE\{subkey}"),
        RegHive::Hkcu => format!(r"Computer\HKEY_CURRENT_USER\{subkey}"),
    };
    if let Ok((k, _)) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Applets\Regedit")
    {
        let _ = k.set_value("LastKey", &full_key);
    }
    Command::new("regedit.exe")
        .arg("-m")
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed launching regedit.exe: {e}"))
}

pub fn extract_safe_app_tokens(display_name: &str, _publisher: &str) -> Vec<String> {
    // Strip trailing version numbers, "(64-bit)", "(x64)", etc.
    let cleaned = display_name
        .split(['(', '[', '-'])
        .next()
        .unwrap_or(display_name)
        .trim();

    let mut tokens = Vec::new();
    let lower = cleaned.to_ascii_lowercase();

    // Remove trailing version token if present (e.g. "7-Zip 24.08" -> "7-Zip")
    let words: Vec<&str> = cleaned
        .split_whitespace()
        .take_while(|w| !w.chars().next().map(|c| c.is_ascii_digit() && w.contains('.')).unwrap_or(false))
        .collect();
    let base_phrase = words.join(" ").trim().to_ascii_lowercase();

    for candidate in [lower, base_phrase] {
        if candidate.len() >= 4
            && !GENERIC_PROTECTED_TOKENS
                .iter()
                .any(|g| candidate.eq_ignore_ascii_case(g))
            && !tokens.contains(&candidate)
        {
            tokens.push(candidate.clone());
            let no_space = candidate.replace(' ', "");
            if no_space.len() >= 5 && !tokens.contains(&no_space) {
                tokens.push(no_space);
            }
        }
    }
    tokens
}

pub fn matches_app_token_strictly(candidate_name: &str, tokens: &[String]) -> bool {
    let clean_cand = candidate_name
        .trim()
        .trim_end_matches(".lnk")
        .trim_end_matches(".url")
        .to_ascii_lowercase();
    if clean_cand.len() < 4 {
        return false;
    }
    if GENERIC_PROTECTED_TOKENS
        .iter()
        .any(|g| clean_cand.eq_ignore_ascii_case(g))
    {
        return false;
    }

    let cand_no_space = clean_cand.replace(' ', "");
    tokens.iter().any(|t| {
        clean_cand == *t
            || cand_no_space == *t
            || clean_cand.starts_with(&format!("{t} "))
            || clean_cand.starts_with(&format!("{t}-"))
            || clean_cand.ends_with(&format!("\\{t}.exe"))
    })
}

/// Ensures we NEVER target a root directory (`C:\`, `C:\Program Files`, `C:\Windows`, `%APPDATA%` root).
pub fn is_safe_specific_folder(path: &Path) -> bool {
    let comps: Vec<_> = path.components().collect();
    if comps.len() < 3 {
        return false;
    }
    let s = path.to_string_lossy().to_ascii_lowercase();
    let trimmed = s.trim_end_matches('\\');
    for forbidden in [
        r"c:\windows",
        r"c:\windows\system32",
        r"c:\windows\syswow64",
        r"c:\program files",
        r"c:\program files (x86)",
        r"c:\programdata",
        r"c:\users",
    ] {
        if trimmed == forbidden {
            return false;
        }
    }
    let leaf = path
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    !GENERIC_PROTECTED_TOKENS
        .iter()
        .any(|g| leaf.eq_ignore_ascii_case(g))
}

fn shallow_folder_size(dir: &Path, max_depth: usize) -> u64 {
    if max_depth == 0 || is_reparse_point(dir) {
        return 0;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut sum = 0u64;
    for ent in entries.flatten().take(250) {
        let p = ent.path();
        if is_reparse_point(&p) {
            continue;
        }
        if let Ok(m) = std::fs::symlink_metadata(&p) {
            if m.is_file() {
                sum = sum.saturating_add(m.len());
            } else if m.is_dir() {
                sum = sum.saturating_add(shallow_folder_size(&p, max_depth - 1));
            }
        }
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geek_leftover_matcher_safety_tests() {
        // Must NEVER produce generic protected tokens like "Microsoft" or "Windows"
        let bad = extract_safe_app_tokens("Windows", "Microsoft");
        assert!(bad.is_empty());

        let vlc = extract_safe_app_tokens("VLC media player (64-bit)", "VideoLAN");
        assert!(matches_app_token_strictly("VLC media player", &vlc));
        assert!(matches_app_token_strictly("VLC media player.lnk", &vlc));
        assert!(!matches_app_token_strictly("Windows", &vlc));
        assert!(!matches_app_token_strictly("Microsoft", &vlc));

        assert!(!is_safe_specific_folder(Path::new(r"C:\Program Files")));
        assert!(!is_safe_specific_folder(Path::new(r"C:\Windows")));
        assert!(!is_safe_specific_folder(Path::new(r"C:\Program Files\Common Files")));
        assert!(is_safe_specific_folder(Path::new(r"C:\Program Files\VideoLAN\VLC")));
    }
}
