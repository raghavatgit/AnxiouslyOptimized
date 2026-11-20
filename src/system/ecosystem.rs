use crate::engine::run_command_timeout;
use crate::models::{EcosystemInfo, ValorantInstallStatus};
use std::path::Path;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY};
use winreg::RegKey;

pub fn detect_ecosystem() -> EcosystemInfo {
    let (val_status, val_path, riot_path) = detect_valorant_quick();

    let (bs_ver, msi_ver, ld_installed) = detect_emulators_quick();
    let steam_installed = is_steam_installed();
    let epic_installed = is_epic_installed();
    let (winget_available, winget_version) = probe_winget();

    EcosystemInfo {
        valorant_status: val_status,
        valorant_path: val_path,
        riot_client_path: riot_path,
        steam_installed,
        epic_installed,
        bluestacks_version: bs_ver,
        msi_player_version: msi_ver,
        ldplayer_installed: ld_installed,
        winget_available,
        winget_version,
    }
}

pub fn is_process_running_ci(exe_name_no_ext_or_exe: &str) -> bool {
    let target = exe_name_no_ext_or_exe
        .trim_end_matches(".exe")
        .to_ascii_lowercase();
    let mut sys = sysinfo::System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    sys.processes().values().any(|p| {
        let name = p
            .name()
            .to_string_lossy()
            .trim_end_matches(".exe")
            .to_ascii_lowercase();
        name == target
    })
}

pub fn detect_valorant_quick() -> (ValorantInstallStatus, Option<String>, Option<String>) {
    let running = is_process_running_ci("VALORANT-Win64-Shipping");
    let mut install_path: Option<String> = None;
    let mut riot_client: Option<String> = None;

    // 1. Check RiotClientInstalls.json
    if let Some(pd) = std::env::var_os("ProgramData") {
        let rc_json = Path::new(&pd).join(r"Riot Games\RiotClientInstalls.json");
        if let Ok(raw) = std::fs::read_to_string(&rc_json) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) {
                for key in ["rc_default", "rc_live", "rc_beta"] {
                    if let Some(s) = val.get(key).and_then(|v| v.as_str()) {
                        if Path::new(s).exists() {
                            riot_client = Some(s.to_string());
                            break;
                        }
                    }
                }
            }
        }
    }

    // 2. Standard paths across ProgramFiles & C:/D:/E:
    let mut candidates = vec![
        r"C:\Riot Games\VALORANT\live".to_string(),
        r"D:\Riot Games\VALORANT\live".to_string(),
        r"E:\Riot Games\VALORANT\live".to_string(),
    ];
    if let Ok(pf) = std::env::var("ProgramFiles") {
        candidates.push(format!(r"{pf}\Riot Games\VALORANT\live"));
    }
    if let Ok(pfx86) = std::env::var("ProgramFiles(x86)") {
        candidates.push(format!(r"{pfx86}\Riot Games\VALORANT\live"));
    }
    for dir in candidates {
        let exe = Path::new(&dir).join("VALORANT.exe");
        if exe.exists() {
            install_path = Some(dir);
            break;
        }
    }

    // 3. Uninstall registry lookup (HKCU + HKLM)
    if install_path.is_none() {
        for (hive, flags) in [
            (HKEY_CURRENT_USER, KEY_READ),
            (HKEY_LOCAL_MACHINE, KEY_READ | KEY_WOW64_64KEY),
            (HKEY_LOCAL_MACHINE, KEY_READ | KEY_WOW64_32KEY),
        ] {
            if let Ok(k) = RegKey::predef(hive).open_subkey_with_flags(
                r"Software\Microsoft\Windows\CurrentVersion\Uninstall\Riot Game valorant.live",
                flags,
            ) {
                if let Ok(loc) = k.get_value::<String, _>("InstallLocation") {
                    if Path::new(&loc).exists() {
                        install_path = Some(loc);
                        break;
                    }
                }
            }
        }
    }

    if riot_client.is_none() {
        for rc in [
            r"C:\Riot Games\Riot Client\RiotClientServices.exe",
            r"D:\Riot Games\Riot Client\RiotClientServices.exe",
        ] {
            if Path::new(rc).exists() {
                riot_client = Some(rc.to_string());
                break;
            }
        }
    }

    let status = if running {
        ValorantInstallStatus::Running
    } else if install_path.is_some() {
        ValorantInstallStatus::InstalledNotRunning
    } else {
        ValorantInstallStatus::NotInstalled
    };

    (status, install_path, riot_client)
}

fn detect_emulators_quick() -> (Option<String>, Option<String>, bool) {
    let pd = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".to_string());
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    let bs_conf = Path::new(&pd).join(r"BlueStacks_nxt\bluestacks.conf");
    let bs_installed = bs_conf.exists()
        || Path::new(r"C:\Program Files\BlueStacks_nxt\HD-Player.exe").exists()
        || Path::new(r"C:\Program Files (x86)\BlueStacks_nxt\HD-Player.exe").exists();
    let bs_ver = if bs_installed {
        let ver = hklm
            .open_subkey_with_flags(r"SOFTWARE\BlueStacks_nxt", KEY_READ | KEY_WOW64_64KEY)
            .and_then(|k| k.get_value::<String, _>("Version"))
            .ok();
        Some(ver.unwrap_or_else(|| "Installed".to_string()))
    } else {
        None
    };

    let msi_conf = Path::new(&pd).join(r"BlueStacks_msi5\bluestacks.conf");
    let msi_installed = msi_conf.exists()
        || Path::new(r"C:\Program Files\BlueStacks_msi5\HD-Player.exe").exists()
        || Path::new(r"C:\Program Files (x86)\BlueStacks_msi5\HD-Player.exe").exists();
    let msi_ver = if msi_installed {
        let ver = hklm
            .open_subkey_with_flags(r"SOFTWARE\BlueStacks_msi5", KEY_READ | KEY_WOW64_64KEY)
            .and_then(|k| k.get_value::<String, _>("Version"))
            .ok();
        Some(ver.unwrap_or_else(|| "Installed".to_string()))
    } else {
        None
    };

    let ld_installed = Path::new(r"C:\LDPlayer\LDPlayer9\dnplayer.exe").exists()
        || Path::new(r"D:\LDPlayer\LDPlayer9\dnplayer.exe").exists();

    (bs_ver, msi_ver, ld_installed)
}

pub fn is_steam_installed() -> bool {
    if let Ok(k) = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(r"Software\Valve\Steam", KEY_READ)
    {
        if let Ok(exe) = k.get_value::<String, _>("SteamExe") {
            if Path::new(&exe).exists() {
                return true;
            }
        }
        if let Ok(path) = k.get_value::<String, _>("SteamPath") {
            if Path::new(&path).join("steam.exe").exists() {
                return true;
            }
        }
    }
    Path::new(r"C:\Program Files (x86)\Steam\steam.exe").exists()
}

pub fn is_epic_installed() -> bool {
    Path::new(r"C:\Program Files (x86)\Epic Games\Launcher\Portal\Binaries\Win64\EpicGamesLauncher.exe")
        .exists()
        || Path::new(r"C:\Program Files\Epic Games\Launcher\Portal\Binaries\Win64\EpicGamesLauncher.exe")
            .exists()
}

pub fn probe_winget() -> (bool, Option<String>) {
    // Wait up to 6s so cold-start winget isn't falsely marked missing
    if let Ok(out) = run_command_timeout("winget.exe", &["--version"], 6000) {
        if out.exit_code == 0 {
            let v = out.stdout.trim().to_string();
            return (true, if v.is_empty() { None } else { Some(v) });
        }
    }
    (false, None)
}
