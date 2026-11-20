#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod engine;
mod models;
mod system;
mod ui;

use crate::engine::updater::APP_VERSION;
use crate::engine::{
    emulator, format_bytes, installer, is_process_elevated, maintenance, safety, to_wide_null,
    tweaks, uninstaller, valorant,
};
use crate::models::SystemReport;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE};
use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};

const APP_ICON_PNG: &[u8] = include_bytes!("../assets/app_icon.png");

struct SingleInstanceGuard(HANDLE);

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}

fn acquire_single_instance() -> Option<SingleInstanceGuard> {
    let mutex_name = to_wide_null(r"Local\OptimizerByRaghav_SingleInstance_Mutex");
    unsafe {
        let Ok(handle) = CreateMutexW(None, false, PCWSTR(mutex_name.as_ptr())) else {
            return None;
        };
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let _ = CloseHandle(handle);
            return None;
        }
        Some(SingleInstanceGuard(handle))
    }
}

fn load_app_icon() -> Option<(Vec<u8>, u32, u32)> {
    let img = image::load_from_memory(APP_ICON_PNG).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    Some((img.into_raw(), w, h))
}

fn run_readonly_self_test() -> i32 {
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
    println!("============================================================");
    println!(" Optimizer by Raghav v{APP_VERSION} — Read-Only Self-Test Mode");
    println!("============================================================");
    println!(
        "Privilege: {}",
        if is_process_elevated() {
            "Administrator (Elevated)"
        } else {
            "Standard User"
        }
    );

    let report = SystemReport::collect();
    println!("{}", report.to_markdown_summary());

    let mut tw = tweaks::catalog_for_os(report.platform.is_win11);
    tweaks::audit_tweaks(&mut tw);
    let applied = tw.iter().filter(|t| t.is_applied).count();
    println!(
        "## Tweaks Audit: {} / {} applied (read-only)",
        applied,
        tw.len()
    );

    let vss = safety::check_vss_health();
    println!("## VSS Health: {}", vss.summary);

    let val = valorant::detect_topology();
    println!(
        "## VALORANT Status: {:?} | Configs: {}",
        val.install_status,
        val.config_paths.len()
    );

    let emu = emulator::detect_topology();
    println!(
        "## Emulators: BlueStacks={} MSI={} LDPlayer={} | IFEO={} | {}",
        emu.bluestacks_installed,
        emu.msi_installed,
        emu.ldplayer_installed,
        emu.ifeo_priority_locked,
        emu.hypervisor_status
    );

    let mr = maintenance::query_mem_reduct_stats();
    println!(
        "## Mem Reduct Telemetry: Phys {}% ({} free) | Commit {}% | System Cache {}",
        mr.phys_load_pct,
        format_bytes(mr.phys_free_bytes),
        mr.commit_load_pct,
        format_bytes(mr.system_cache_bytes)
    );

    let apps = uninstaller::scan_all_installed_apps(false);
    println!("## Geek Uninstaller: {} Win32 installed programs indexed", apps.len());

    println!("SELF-TEST PASSED (0 mutations performed).");
    0
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a.eq_ignore_ascii_case("--self-test")) {
        let code = run_readonly_self_test();
        std::process::exit(code);
    }

    if args.iter().any(|a| a.eq_ignore_ascii_case("--install")) {
        let quiet = args.iter().any(|a| a.eq_ignore_ascii_case("--quiet"));
        match installer::install_current_exe_to_system() {
            Ok(p) => {
                if !quiet {
                    let title = to_wide_null("Optimizer by Raghav Setup");
                    let msg = to_wide_null(&format!(
                        "Optimizer by Raghav v{APP_VERSION} installed to:\n{}\n\nDesktop & Start Menu shortcuts created.",
                        p.display()
                    ));
                    unsafe {
                        MessageBoxW(
                            None,
                            PCWSTR(msg.as_ptr()),
                            PCWSTR(title.as_ptr()),
                            MB_OK | MB_ICONINFORMATION,
                        );
                    }
                }
                std::process::exit(0);
            }
            Err(e) => {
                if !quiet {
                    let title = to_wide_null("Optimizer by Raghav Setup Error");
                    let msg = to_wide_null(&e);
                    unsafe {
                        MessageBoxW(
                            None,
                            PCWSTR(msg.as_ptr()),
                            PCWSTR(title.as_ptr()),
                            MB_OK | MB_ICONINFORMATION,
                        );
                    }
                }
                std::process::exit(1);
            }
        }
    }

    if args.iter().any(|a| a.eq_ignore_ascii_case("--uninstall")) {
        let quiet = args.iter().any(|a| a.eq_ignore_ascii_case("--quiet"));
        let res = installer::uninstall_app_from_system();
        if !quiet {
            let title = to_wide_null("Optimizer by Raghav Uninstaller");
            let text = match &res {
                Ok(m) => m.as_str(),
                Err(e) => e.as_str(),
            };
            let msg = to_wide_null(text);
            unsafe {
                MessageBoxW(
                    None,
                    PCWSTR(msg.as_ptr()),
                    PCWSTR(title.as_ptr()),
                    MB_OK | MB_ICONINFORMATION,
                );
            }
        }
        std::process::exit(if res.is_ok() { 0 } else { 1 });
    }

    let Some(_instance_guard) = acquire_single_instance() else {
        let title = to_wide_null("Optimizer by Raghav");
        let msg = to_wide_null("Optimizer by Raghav is already running.");
        unsafe {
            MessageBoxW(
                None,
                PCWSTR(msg.as_ptr()),
                PCWSTR(title.as_ptr()),
                MB_OK | MB_ICONINFORMATION,
            );
        }
        return;
    };

    let icon_rgba = load_app_icon();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(format!("Optimizer by Raghav v{APP_VERSION}"))
        .with_inner_size([1260.0, 820.0])
        .with_min_inner_size([1000.0, 660.0]);

    if let Some((ref rgba, w, h)) = icon_rgba {
        viewport = viewport.with_icon(std::sync::Arc::new(egui::IconData {
            rgba: rgba.clone(),
            width: w,
            height: h,
        }));
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let icon_for_app = icon_rgba;
    if let Err(e) = eframe::run_native(
        "Optimizer by Raghav",
        native_options,
        Box::new(move |cc| Ok(Box::new(app::OptimizerApp::new(cc, icon_for_app)))),
    ) {
        let title = to_wide_null("Optimizer by Raghav - Graphics Error");
        let msg = to_wide_null(&format!("Failed to initialize GPU window: {e}"));
        unsafe {
            MessageBoxW(
                None,
                PCWSTR(msg.as_ptr()),
                PCWSTR(title.as_ptr()),
                MB_OK | MB_ICONINFORMATION,
            );
        }
    }
}
