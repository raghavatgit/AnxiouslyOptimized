use crate::engine::{run_command_streaming, run_command_timeout};
use crate::models::RuntimePackageItem;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY};
use winreg::RegKey;

pub fn get_runtime_catalog() -> Vec<RuntimePackageItem> {
    let mut items = vec![
        RuntimePackageItem {
            id: "Microsoft.VCRedist.2015+.x64".into(),
            name: "Visual C++ 2015-2022 Redistributable (x64)".into(),
            category: "Core Runtimes".into(),
            description: "Required by 64-bit DirectX, Unreal, and Unity games.".into(),
            is_installed: false,
            detected_detail: None,
            is_selected: false,
        },
        RuntimePackageItem {
            id: "Microsoft.VCRedist.2015+.x86".into(),
            name: "Visual C++ 2015-2022 Redistributable (x86)".into(),
            category: "Core Runtimes".into(),
            description: "Required by 32-bit game launchers, anti-cheat helpers, and emulators.".into(),
            is_installed: false,
            detected_detail: None,
            is_selected: false,
        },
        RuntimePackageItem {
            id: "Microsoft.DirectX".into(),
            name: "DirectX End-User Runtimes (June 2010)".into(),
            category: "Core Runtimes".into(),
            description: "Installs legacy d3dx9/10/11 libraries (checked across both System32 & SysWOW64).".into(),
            is_installed: false,
            detected_detail: None,
            is_selected: false,
        },
        RuntimePackageItem {
            id: "Microsoft.DotNet.DesktopRuntime.8".into(),
            name: ".NET Desktop Runtime 8 (LTS)".into(),
            category: "Core Runtimes".into(),
            description: "Modern Microsoft .NET runtime for overlays and utilities.".into(),
            is_installed: false,
            detected_detail: None,
            is_selected: false,
        },
        RuntimePackageItem {
            id: "7zip.7zip".into(),
            name: "7-Zip Archiver".into(),
            category: "Utilities".into(),
            description: "Fast open-source file archiver via official WinGet package.".into(),
            is_installed: false,
            detected_detail: None,
            is_selected: false,
        },
        RuntimePackageItem {
            id: "Guru3D.RTSS".into(),
            name: "RivaTuner Statistics Server (RTSS)".into(),
            category: "Gaming Tools".into(),
            description: "Low-overhead frame rate limiter & frametime telemetry overlay.".into(),
            is_installed: false,
            detected_detail: None,
            is_selected: false,
        },
        RuntimePackageItem {
            id: "OBSProject.OBSStudio".into(),
            name: "OBS Studio".into(),
            category: "Gaming Tools".into(),
            description: "Hardware-accelerated screen recording & streaming.".into(),
            is_installed: false,
            detected_detail: None,
            is_selected: false,
        },
        RuntimePackageItem {
            id: "Valve.Steam".into(),
            name: "Steam Client".into(),
            category: "Platforms".into(),
            description: "Valve digital game client.".into(),
            is_installed: false,
            detected_detail: None,
            is_selected: false,
        },
        RuntimePackageItem {
            id: "Discord.Discord".into(),
            name: "Discord".into(),
            category: "Communication".into(),
            description: "Voice & game community client.".into(),
            is_installed: false,
            detected_detail: None,
            is_selected: false,
        },
        RuntimePackageItem {
            id: "BlueStack.BlueStacks".into(),
            name: "BlueStacks 5".into(),
            category: "Emulators".into(),
            description: "Android gaming emulator.".into(),
            is_installed: false,
            detected_detail: None,
            is_selected: false,
        },
    ];
    refresh_runtime_detection(&mut items);
    items
}

pub fn refresh_runtime_detection(items: &mut [RuntimePackageItem]) {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let windir = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    let pf = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string());
    let pfx86 =
        std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| r"C:\Program Files (x86)".to_string());
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let pd = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".to_string());

    for item in items.iter_mut() {
        let (installed, detail) = match item.id.as_str() {
            "Microsoft.VCRedist.2015+.x64" => {
                detect_vcredist(&hklm, r"SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\X64")
            }
            "Microsoft.VCRedist.2015+.x86" => {
                detect_vcredist(&hklm, r"SOFTWARE\WOW6432Node\Microsoft\VisualStudio\14.0\VC\Runtimes\X86")
                    .or_else_tuple(|| {
                        detect_vcredist(&hklm, r"SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\X86")
                    })
            }
            "Microsoft.DirectX" => {
                let d64 = Path::new(&windir).join(r"System32\d3dx9_43.dll").exists()
                    && Path::new(&windir).join(r"System32\d3dx11_43.dll").exists();
                let d32 = Path::new(&windir).join(r"SysWOW64\d3dx9_43.dll").exists()
                    && Path::new(&windir).join(r"SysWOW64\d3dx11_43.dll").exists();
                if d64 && d32 {
                    (true, Some("System32 + SysWOW64 verified".to_string()))
                } else if d64 {
                    (false, Some("Only 64-bit present (32-bit SysWOW64 missing)".to_string()))
                } else {
                    (false, None)
                }
            }
            "Microsoft.DotNet.DesktopRuntime.8" => {
                let dir = Path::new(&pf).join(r"dotnet\shared\Microsoft.WindowsDesktop.App");
                let mut found_ver = None;
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for e in entries.flatten() {
                        let name = e.file_name().to_string_lossy().to_string();
                        if name.starts_with("8.") || name.starts_with("9.") || name.starts_with("10.") {
                            found_ver = Some(format!("v{name}"));
                            break;
                        }
                    }
                }
                (found_ver.is_some(), found_ver)
            }
            "7zip.7zip" => {
                let p64 = Path::new(&pf).join(r"7-Zip\7z.exe");
                let p86 = Path::new(&pfx86).join(r"7-Zip\7z.exe");
                let ok = p64.exists() || p86.exists();
                (ok, if ok { Some("7z.exe verified".into()) } else { None })
            }
            "Guru3D.RTSS" => {
                let p1 = Path::new(&pfx86).join(r"RivaTuner Statistics Server\RTSS.exe");
                let p2 = Path::new(&pf).join(r"RivaTuner Statistics Server\RTSS.exe");
                let ok = p1.exists() || p2.exists();
                (ok, if ok { Some("RTSS.exe verified".into()) } else { None })
            }
            "OBSProject.OBSStudio" => {
                let p1 = Path::new(&pf).join(r"obs-studio\bin\64bit\obs64.exe");
                let p2 = find_in_uninstall(&hklm, &hkcu, "OBS Studio");
                let ok = p1.exists() || p2.is_some();
                (ok, p2.or_else(|| if p1.exists() { Some("obs64.exe verified".into()) } else { None }))
            }
            "Valve.Steam" => {
                let reg_exe = hkcu
                    .open_subkey_with_flags(r"Software\Valve\Steam", KEY_READ)
                    .and_then(|k| k.get_value::<String, _>("SteamExe"))
                    .ok()
                    .filter(|p| Path::new(p).exists());
                let def = Path::new(&pfx86).join(r"Steam\steam.exe");
                let ok = reg_exe.is_some() || def.exists();
                (ok, if ok { Some("steam.exe verified".into()) } else { None })
            }
            "Discord.Discord" => {
                let d = Path::new(&local).join(r"Discord\Update.exe");
                let ok = d.exists();
                (ok, if ok { Some("Update.exe verified".into()) } else { None })
            }
            "BlueStack.BlueStacks" => {
                let ok = Path::new(&pd).join(r"BlueStacks_nxt\bluestacks.conf").exists()
                    || Path::new(&pf).join(r"BlueStacks_nxt\HD-Player.exe").exists()
                    || Path::new(&pfx86).join(r"BlueStacks_nxt\HD-Player.exe").exists();
                let ver = hklm
                    .open_subkey_with_flags(r"SOFTWARE\BlueStacks_nxt", KEY_READ | KEY_WOW64_64KEY)
                    .and_then(|k| k.get_value::<String, _>("Version"))
                    .ok();
                (ok, ver.map(|v| format!("v{v}")))
            }
            _ => (false, None),
        };
        item.is_installed = installed;
        item.detected_detail = detail;
    }
}

trait TupleFallback {
    fn or_else_tuple(self, f: impl FnOnce() -> (bool, Option<String>)) -> (bool, Option<String>);
}
impl TupleFallback for (bool, Option<String>) {
    fn or_else_tuple(self, f: impl FnOnce() -> (bool, Option<String>)) -> (bool, Option<String>) {
        if self.0 {
            self
        } else {
            f()
        }
    }
}

fn detect_vcredist(hklm: &RegKey, subkey: &str) -> (bool, Option<String>) {
    if let Ok(key) = hklm.open_subkey_with_flags(subkey, KEY_READ | KEY_WOW64_64KEY) {
        let installed: u32 = key.get_value("Installed").unwrap_or(0);
        if installed == 1 {
            let ver: String = key.get_value("Version").unwrap_or_default();
            let detail = if ver.is_empty() {
                "Installed".to_string()
            } else {
                ver
            };
            return (true, Some(detail));
        }
    }
    (false, None)
}

fn find_in_uninstall(hklm: &RegKey, hkcu: &RegKey, display_prefix: &str) -> Option<String> {
    let prefix_l = display_prefix.to_ascii_lowercase();
    for (root, flags) in [
        (hklm, KEY_READ | KEY_WOW64_64KEY),
        (hklm, KEY_READ | KEY_WOW64_32KEY),
        (hkcu, KEY_READ),
    ] {
        if let Ok(uninst) =
            root.open_subkey_with_flags(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall", flags)
        {
            for sub_name in uninst.enum_keys().flatten() {
                if let Ok(sub) = uninst.open_subkey_with_flags(&sub_name, flags) {
                    let name: String = sub.get_value("DisplayName").unwrap_or_default();
                    if name.to_ascii_lowercase().contains(&prefix_l) {
                        let ver: String = sub.get_value("DisplayVersion").unwrap_or_default();
                        return Some(if ver.is_empty() { name } else { format!("v{ver}") });
                    }
                }
            }
        }
    }
    None
}

pub fn install_package_via_winget<F>(
    id: &str,
    on_line: F,
    cancel: &AtomicBool,
) -> Result<(), String>
where
    F: Fn(String) + Send + Sync + 'static,
{
    let ver_check = run_command_timeout("winget.exe", &["--version"], 6000);
    if !matches!(ver_check, Ok(o) if o.exit_code == 0) {
        return Err(
            "WinGet CLI is unavailable on this system. Install 'App Installer' from Microsoft Store first."
                .to_string(),
        );
    }

    let code = run_command_streaming(
        "winget.exe",
        &[
            "install",
            "--id",
            id,
            "--exact",
            "--silent",
            "--accept-package-agreements",
            "--accept-source-agreements",
            "--disable-interactivity",
        ],
        false,
        on_line,
        cancel,
        300_000,
    )?;

    if code == 0 {
        Ok(())
    } else {
        Err(format!(
            "winget install '{id}' exited with code {code} (0x{:08X})",
            code as u32
        ))
    }
}
