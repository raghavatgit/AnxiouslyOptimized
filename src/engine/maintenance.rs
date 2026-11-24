use crate::engine::{run_command_streaming, run_command_timeout, system_drive_root, to_wide_null};
use crate::models::{MemReductConfig, MemReductLiveStats};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use windows::core::{s, w, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE, LUID};
use windows::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED,
    TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
use windows::Win32::System::Memory::SetSystemFileCacheSize;
use windows::Win32::System::ProcessStatus::{GetPerformanceInfo, PERFORMANCE_INFORMATION};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct StandbyPurgeResult {
    pub avail_before_bytes: u64,
    pub avail_after_bytes: u64,
    pub used_before_bytes: u64,
    pub used_after_bytes: u64,
    pub cache_before_bytes: u64,
    pub cache_after_bytes: u64,
    pub pages_combined: u64,
    pub regions_cleaned: Vec<String>,
    pub nt_status: i32,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct HibernationStatus {
    pub enabled: bool,
    pub file_path: String,
    pub size_bytes: u64,
}

#[repr(C)]
struct MemoryCombineInformationEx {
    handle: usize,
    pages_combined: usize,
    flags: u32,
}

fn enable_privilege(priv_str: &str) -> Result<(), String> {
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token,
        )
        .map_err(|e| format!("OpenProcessToken failed: {e}"))?;

        let priv_name = to_wide_null(priv_str);
        let mut luid = LUID::default();
        let lookup = LookupPrivilegeValueW(None, PCWSTR(priv_name.as_ptr()), &mut luid);
        if let Err(e) = lookup {
            let _ = CloseHandle(token);
            return Err(format!("LookupPrivilegeValueW({priv_str}) failed: {e}"));
        }

        let tp = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };

        let adj = AdjustTokenPrivileges(token, false, Some(&tp), 0, None, None);
        let last_err = windows::Win32::Foundation::GetLastError();
        let _ = CloseHandle(token);

        adj.map_err(|e| format!("AdjustTokenPrivileges({priv_str}) failed: {e}"))?;
        if last_err.0 != 0 {
            return Err(format!(
                "{priv_str} not held (error {}). Run as Administrator.",
                last_err.0
            ));
        }
        Ok(())
    }
}

pub fn query_mem_reduct_stats() -> MemReductLiveStats {
    unsafe {
        let mut stats = MemReductLiveStats::default();
        let mut mem = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };
        if GlobalMemoryStatusEx(&mut mem).is_ok() {
            stats.phys_total_bytes = mem.ullTotalPhys;
            stats.phys_free_bytes = mem.ullAvailPhys;
            stats.phys_used_bytes = mem.ullTotalPhys.saturating_sub(mem.ullAvailPhys);
            stats.phys_load_pct = mem.dwMemoryLoad;
        }

        let mut perf = PERFORMANCE_INFORMATION {
            cb: std::mem::size_of::<PERFORMANCE_INFORMATION>() as u32,
            ..Default::default()
        };
        if GetPerformanceInfo(&mut perf, perf.cb).is_ok() {
            let ps = perf.PageSize as u64;
            stats.system_cache_bytes = (perf.SystemCache as u64).saturating_mul(ps);
            stats.kernel_paged_bytes = (perf.KernelPaged as u64).saturating_mul(ps);
            stats.kernel_nonpaged_bytes = (perf.KernelNonpaged as u64).saturating_mul(ps);
            stats.commit_total_bytes = (perf.CommitTotal as u64).saturating_mul(ps);
            stats.commit_limit_bytes = (perf.CommitLimit as u64).saturating_mul(ps);
            stats.commit_free_bytes = stats
                .commit_limit_bytes
                .saturating_sub(stats.commit_total_bytes);
            if stats.commit_limit_bytes > 0 {
                stats.commit_load_pct =
                    ((stats.commit_total_bytes as f64 / stats.commit_limit_bytes as f64) * 100.0)
                        .round() as u32;
            }
        }
        stats
    }
}

/// Full Mem Reduct multi-region kernel memory cleaner:
/// - Working Sets (`SystemMemoryListInformation=80`, `MemoryEmptyWorkingSets=2`)
/// - System File Cache (`SetSystemFileCacheSize(!0, !0, 0)`)
/// - Modified Page List (`MemoryFlushModifiedList=3`)
/// - Standby List (`MemoryPurgeStandbyList=4`)
/// - Priority-0 Low-Priority Standby (`MemoryPurgeLowPriorityStandbyList=5`)
/// - Combine Physical Memory Pages (`SystemCombinePhysicalMemoryInformation=130`)
pub fn clean_memory_reduct(cfg: &MemReductConfig) -> Result<StandbyPurgeResult, String> {
    enable_privilege("SeProfileSingleProcessPrivilege")?;
    let _ = enable_privilege("SeIncreaseQuotaPrivilege");
    let _ = enable_privilege("SeDebugPrivilege");

    type NtSetSystemInformationFn =
        unsafe extern "system" fn(u32, *mut core::ffi::c_void, u32) -> i32;
    const SYSTEM_MEMORY_LIST_INFORMATION: u32 = 80;
    const SYSTEM_COMBINE_PHYSICAL_MEMORY_INFORMATION: u32 = 130;

    const MEMORY_EMPTY_WORKING_SETS: u32 = 2;
    const MEMORY_FLUSH_MODIFIED_LIST: u32 = 3;
    const MEMORY_PURGE_STANDBY_LIST: u32 = 4;
    const MEMORY_PURGE_LOW_PRIORITY_STANDBY_LIST: u32 = 5;

    let before = query_mem_reduct_stats();
    let mut regions_cleaned = Vec::new();
    let mut last_nt_status: i32 = 0;
    let mut pages_combined: u64 = 0;

    unsafe {
        let hmod = GetModuleHandleW(w!("ntdll.dll"))
            .map_err(|e| format!("GetModuleHandleW(ntdll.dll) failed: {e}"))?;
        let addr = GetProcAddress(hmod, s!("NtSetSystemInformation"))
            .ok_or_else(|| "NtSetSystemInformation export not found".to_string())?;
        let nt_set: NtSetSystemInformationFn = std::mem::transmute(addr);

        // 1. Working Sets (moves inactive working set pages to standby)
        if cfg.clean_working_sets {
            let mut cmd = MEMORY_EMPTY_WORKING_SETS;
            let st = nt_set(
                SYSTEM_MEMORY_LIST_INFORMATION,
                &mut cmd as *mut u32 as *mut _,
                std::mem::size_of::<u32>() as u32,
            );
            last_nt_status = st;
            if st == 0 {
                regions_cleaned.push("Working Sets".to_string());
            }
        }

        // 2. System File Cache Working Set
        if cfg.clean_system_file_cache {
            if SetSystemFileCacheSize(usize::MAX, usize::MAX, 0).is_ok() {
                regions_cleaned.push("System File Cache".to_string());
            }
        }

        // 3. Modified Page List (flushes dirty pages to disk -> moves to standby)
        if cfg.clean_modified_page_list {
            let mut cmd = MEMORY_FLUSH_MODIFIED_LIST;
            let st = nt_set(
                SYSTEM_MEMORY_LIST_INFORMATION,
                &mut cmd as *mut u32 as *mut _,
                std::mem::size_of::<u32>() as u32,
            );
            if st == 0 {
                regions_cleaned.push("Modified Page List".to_string());
            }
        }

        // 4. Low-Priority Standby List (Priority 0)
        if cfg.clean_low_priority_standby && !cfg.clean_standby_list {
            let mut cmd = MEMORY_PURGE_LOW_PRIORITY_STANDBY_LIST;
            let st = nt_set(
                SYSTEM_MEMORY_LIST_INFORMATION,
                &mut cmd as *mut u32 as *mut _,
                std::mem::size_of::<u32>() as u32,
            );
            last_nt_status = st;
            if st == 0 {
                regions_cleaned.push("Standby (Low-Priority)".to_string());
            }
        }

        // 5. Full Standby List (purges all standby pages including those just evicted from working set / modified list)
        if cfg.clean_standby_list {
            let mut cmd = MEMORY_PURGE_STANDBY_LIST;
            let st = nt_set(
                SYSTEM_MEMORY_LIST_INFORMATION,
                &mut cmd as *mut u32 as *mut _,
                std::mem::size_of::<u32>() as u32,
            );
            last_nt_status = st;
            if st == 0 {
                regions_cleaned.push("Standby List".to_string());
            }
        }

        // 6. Combine Physical Memory Lists (Win10/11 page deduplication)
        if cfg.clean_combine_memory_lists {
            let mut combine = MemoryCombineInformationEx {
                handle: 0,
                pages_combined: 0,
                flags: 0,
            };
            let st = nt_set(
                SYSTEM_COMBINE_PHYSICAL_MEMORY_INFORMATION,
                &mut combine as *mut _ as *mut _,
                std::mem::size_of::<MemoryCombineInformationEx>() as u32,
            );
            if st == 0 {
                pages_combined = combine.pages_combined as u64;
                regions_cleaned.push(format!("Combined ({pages_combined} pages)"));
            }
        }
    }

    if regions_cleaned.is_empty() {
        return Err(format!(
            "No memory regions could be cleaned (last NTSTATUS 0x{:08X}). Select at least one region and run as Administrator.",
            last_nt_status as u32
        ));
    }

    std::thread::sleep(std::time::Duration::from_millis(90));
    let after = query_mem_reduct_stats();

    Ok(StandbyPurgeResult {
        avail_before_bytes: before.phys_free_bytes,
        avail_after_bytes: after.phys_free_bytes,
        used_before_bytes: before.phys_used_bytes,
        used_after_bytes: after.phys_used_bytes,
        cache_before_bytes: before.system_cache_bytes,
        cache_after_bytes: after.system_cache_bytes,
        pages_combined,
        regions_cleaned,
        nt_status: last_nt_status,
    })
}

pub fn empty_recycle_bin_silent() -> Result<(), String> {
    type ShEmptyRecycleBinWFn = unsafe extern "system" fn(usize, PCWSTR, u32) -> i32;
    const SHERB_NOCONFIRMATION: u32 = 0x0000_0001;
    const SHERB_NOPROGRESSUI: u32 = 0x0000_0002;
    const SHERB_NOSOUND: u32 = 0x0000_0004;

    unsafe {
        let hmod = LoadLibraryW(w!("shell32.dll"))
            .map_err(|e| format!("LoadLibraryW(shell32.dll) failed: {e}"))?;
        let addr = GetProcAddress(hmod, s!("SHEmptyRecycleBinW"))
            .ok_or_else(|| "SHEmptyRecycleBinW export missing".to_string())?;
        let func: ShEmptyRecycleBinWFn = std::mem::transmute(addr);
        let _ = func(
            0,
            PCWSTR(std::ptr::null()),
            SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND,
        );
    }
    Ok(())
}

pub fn clear_all_windows_event_logs() -> Result<usize, String> {
    let list_out = run_command_timeout("wevtutil.exe", &["el"], 10_000)?;
    if list_out.exit_code != 0 {
        return Err(format!("wevtutil el failed: {}", list_out.stderr.trim()));
    }
    let mut cleared = 0usize;
    for log_name in list_out.stdout.lines().map(str::trim).filter(|s| !s.is_empty()) {
        if let Ok(o) = run_command_timeout("wevtutil.exe", &["cl", log_name], 2500) {
            if o.exit_code == 0 {
                cleared += 1;
            }
        }
    }
    Ok(cleared)
}

pub fn get_hibernation_status() -> HibernationStatus {
    let drive = system_drive_root();
    let file_path = format!("{drive}hiberfil.sys");
    match std::fs::metadata(Path::new(&file_path)) {
        Ok(meta) => HibernationStatus {
            enabled: true,
            file_path,
            size_bytes: meta.len(),
        },
        Err(_) => HibernationStatus {
            enabled: false,
            file_path,
            size_bytes: 0,
        },
    }
}

pub fn toggle_hibernation(enable: bool) -> Result<HibernationStatus, String> {
    let arg = if enable { "on" } else { "off" };
    let out = run_command_timeout("powercfg.exe", &["/hibernate", arg], 8000)?;
    if out.exit_code != 0 {
        return Err(format!(
            "powercfg /hibernate {arg} failed (exit {}): {} {}",
            out.exit_code,
            out.stdout.trim(),
            out.stderr.trim()
        ));
    }
    Ok(get_hibernation_status())
}

pub fn run_sfc_scan<F>(on_line: F, cancel: &AtomicBool) -> Result<i32, String>
where
    F: Fn(String) + Send + Sync + 'static,
{
    run_command_streaming(
        "sfc.exe",
        &["/scannow"],
        true, // sfc outputs UTF-16LE!
        on_line,
        cancel,
        1_200_000,
    )
}

pub fn run_dism_restore_health<F>(on_line: F, cancel: &AtomicBool) -> Result<i32, String>
where
    F: Fn(String) + Send + Sync + 'static,
{
    run_command_streaming(
        "dism.exe",
        &["/Online", "/Cleanup-Image", "/RestoreHealth"],
        false,
        on_line,
        cancel,
        1_800_000,
    )
}

pub fn run_dism_component_cleanup<F>(
    reset_base: bool,
    on_line: F,
    cancel: &AtomicBool,
) -> Result<i32, String>
where
    F: Fn(String) + Send + Sync + 'static,
{
    let mut args = vec!["/Online", "/Cleanup-Image", "/StartComponentCleanup"];
    if reset_base {
        args.push("/ResetBase");
    }
    run_command_streaming("dism.exe", &args, false, on_line, cancel, 1_800_000)
}
