use crate::engine::updater::APP_VERSION;
use crate::engine::{run_powershell_encoded, CREATE_NO_WINDOW};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY, KEY_WRITE};
use winreg::RegKey;

const UNINSTALL_REG_PATH: &str =
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\OptimizerByRaghav";

pub fn default_install_dir() -> PathBuf {
    let pf = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string());
    Path::new(&pf).join("Optimizer by Raghav")
}

pub fn installed_exe_path() -> PathBuf {
    default_install_dir().join("OptimizerByRaghav.exe")
}

pub fn is_app_installed() -> bool {
    let exe_ok = installed_exe_path().is_file();
    let reg_ok = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(UNINSTALL_REG_PATH, KEY_READ | KEY_WOW64_64KEY)
        .is_ok();
    exe_ok && reg_ok
}

/// Installs Optimizer by Raghav to `%ProgramFiles%\Optimizer by Raghav\OptimizerByRaghav.exe`,
/// creates Desktop & Start Menu `.lnk` shortcuts, and registers Add/Remove Programs (`Uninstall\OptimizerByRaghav`).
pub fn install_app_bytes(
    payload_bytes: &[u8],
    create_desktop_shortcut: bool,
    create_start_menu_shortcut: bool,
) -> Result<PathBuf, String> {
    let install_dir = default_install_dir();
    std::fs::create_dir_all(&install_dir)
        .map_err(|e| format!("Failed creating {}: {e} (Run as Administrator)", install_dir.display()))?;

    let target_exe = installed_exe_path();
    // If existing installed exe is running, rename it to a temp bak so overwrite succeeds cleanly
    if target_exe.exists() && std::fs::write(&target_exe, payload_bytes).is_err() {
        let old_bak = install_dir.join(".OptimizerByRaghav.old.exe");
        let _ = std::fs::remove_file(&old_bak);
        let _ = std::fs::rename(&target_exe, &old_bak);
    }

    std::fs::write(&target_exe, payload_bytes).map_err(|e| {
        format!(
            "Failed writing {}: {e}",
            target_exe.display()
        )
    })?;

    // Register in HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\OptimizerByRaghav
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let (uninst_key, _) = hklm
        .create_subkey_with_flags(UNINSTALL_REG_PATH, KEY_WRITE | KEY_WOW64_64KEY)
        .map_err(|e| format!("Failed registering Uninstall registry key: {e}"))?;

    let exe_str = target_exe.to_string_lossy().to_string();
    let dir_str = install_dir.to_string_lossy().to_string();
    let icon_str = format!("{exe_str},0");
    let uninst_cmd = format!("\"{exe_str}\" --uninstall");
    let quiet_uninst_cmd = format!("\"{exe_str}\" --uninstall --quiet");
    let size_kb = ((payload_bytes.len() as u64) / 1024).max(1) as u32;

    let _ = uninst_key.set_value("DisplayName", &"Optimizer by Raghav");
    let _ = uninst_key.set_value("DisplayVersion", &APP_VERSION);
    let _ = uninst_key.set_value("Publisher", &"Raghav Goyal");
    let _ = uninst_key.set_value("DisplayIcon", &icon_str);
    let _ = uninst_key.set_value("InstallLocation", &dir_str);
    let _ = uninst_key.set_value("UninstallString", &uninst_cmd);
    let _ = uninst_key.set_value("QuietUninstallString", &quiet_uninst_cmd);
    let _ = uninst_key.set_value("EstimatedSize", &size_kb);
    let _ = uninst_key.set_value("NoModify", &1u32);
    let _ = uninst_key.set_value("NoRepair", &1u32);

    // Create Start Menu & Desktop shortcuts via WScript.Shell COM
    let mut shortcut_targets = Vec::new();
    if create_start_menu_shortcut {
        let pd = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".to_string());
        let sm_path = Path::new(&pd).join(r"Microsoft\Windows\Start Menu\Programs\Optimizer by Raghav.lnk");
        shortcut_targets.push(sm_path.to_string_lossy().to_string());
    }
    if create_desktop_shortcut {
        if let Ok(user_prof) = std::env::var("USERPROFILE") {
            let d_path = Path::new(&user_prof).join(r"Desktop\Optimizer by Raghav.lnk");
            shortcut_targets.push(d_path.to_string_lossy().to_string());
        }
    }

    if !shortcut_targets.is_empty() {
        let exe_escaped = exe_str.replace('\'', "''");
        let dir_escaped = dir_str.replace('\'', "''");
        let mut ps = String::from("$ws = New-Object -ComObject WScript.Shell; ");
        for lnk in &shortcut_targets {
            let lnk_escaped = lnk.replace('\'', "''");
            ps.push_str(&format!(
                "$s = $ws.CreateShortcut('{0}'); \
                 $s.TargetPath = '{1}'; \
                 $s.WorkingDirectory = '{2}'; \
                 $s.IconLocation = '{1},0'; \
                 $s.Description = 'Optimizer by Raghav - Windows System Identifier & Performance Optimizer'; \
                 $s.Save(); ",
                lnk_escaped, exe_escaped, dir_escaped
            ));
        }
        let _ = run_powershell_encoded(&ps, 10_000);
    }

    Ok(target_exe)
}

/// Installs the currently running executable into `%ProgramFiles%\Optimizer by Raghav\OptimizerByRaghav.exe`.
pub fn install_current_exe_to_system() -> Result<PathBuf, String> {
    let cur = std::env::current_exe()
        .map_err(|e| format!("Failed resolving current exe path: {e}"))?;
    let bytes = std::fs::read(&cur)
        .map_err(|e| format!("Failed reading {}: {e}", cur.display()))?;
    install_app_bytes(&bytes, true, true)
}

/// Removes shortcuts, removes Uninstall registry registration, and deletes `%ProgramFiles%\Optimizer by Raghav`.
pub fn uninstall_app_from_system() -> Result<String, String> {
    // 1. Remove Shortcuts
    let pd = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".to_string());
    let sm_lnk = Path::new(&pd).join(r"Microsoft\Windows\Start Menu\Programs\Optimizer by Raghav.lnk");
    let _ = std::fs::remove_file(sm_lnk);

    if let Ok(ap) = std::env::var("APPDATA") {
        let user_sm = Path::new(&ap).join(r"Microsoft\Windows\Start Menu\Programs\Optimizer by Raghav.lnk");
        let _ = std::fs::remove_file(user_sm);
    }
    if let Ok(up) = std::env::var("USERPROFILE") {
        let desk_lnk = Path::new(&up).join(r"Desktop\Optimizer by Raghav.lnk");
        let _ = std::fs::remove_file(desk_lnk);
    }
    let pub_desk = Path::new(r"C:\Users\Public\Desktop\Optimizer by Raghav.lnk");
    let _ = std::fs::remove_file(pub_desk);

    // 2. Remove Uninstall Registry Key
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok(parent) = hklm.open_subkey_with_flags(
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        KEY_WRITE | KEY_WOW64_64KEY,
    ) {
        let _ = parent.delete_subkey_all("OptimizerByRaghav");
    }

    // 3. Remove or schedule deletion of `C:\Program Files\Optimizer by Raghav`
    let install_dir = default_install_dir();
    if install_dir.exists() {
        if std::fs::remove_dir_all(&install_dir).is_err() {
            // If currently running from inside install_dir, schedule delayed self-cleanup
            let dir_str = install_dir.to_string_lossy().to_string();
            let cmd_arg = format!(
                "ping 127.0.0.1 -n 2 >nul & rmdir /s /q \"{}\"",
                dir_str
            );
            let _ = Command::new("cmd.exe")
                .args(["/C", &cmd_arg])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
        }
    }

    Ok("Optimizer by Raghav has been uninstalled from Program Files, Desktop, Start Menu, and Installed Apps.".to_string())
}
