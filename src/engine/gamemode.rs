use crate::engine::{gamemode_config_path, push_log, SharedLogs};
use crate::models::{GameModeConfig, GameModeStatus, LogLevel};
use crate::system::platform::{format_guid, get_active_power_scheme, parse_guid, query_timer_resolution_ms};
use crate::engine::tweaks::ensure_or_find_performance_scheme_guid;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use windows::Win32::Foundation::{CloseHandle, FILETIME, INVALID_HANDLE_VALUE};
use windows::Win32::Media::{timeBeginPeriod, timeEndPeriod};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Power::PowerSetActiveScheme;
use windows::Win32::System::Threading::{
    GetPriorityClass, GetProcessTimes, OpenProcess, SetPriorityClass, SetProcessAffinityMask,
    SetProcessInformation, BELOW_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS,
    PROCESS_CREATION_FLAGS, PROCESS_POWER_THROTTLING_CURRENT_VERSION,
    PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_STATE,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_INFORMATION, ProcessPowerThrottling,
};

/// Games protected by kernel anti-cheats (Vanguard, EAC, BattlEye).
/// NEVER open handles or set priority/affinity against these processes — strictly OS-level tuning only.
const ANTICHEAT_PROTECTED: &[&str] = &[
    "valorant-win64-shipping",
    "league of legends",
    "fortniteclient-win64-shipping",
    "r5apex",
    "rainbowsix",
];

const STANDARD_TARGETS: &[&str] = &[
    "hd-player",
    "dnplayer",
    "nox",
    "noxvmhandle",
    "mumuplayer",
    "nemuplayer",
    "cs2",
    "overwatch",
    "gta5",
    "robloxplayerbeta",
];

const BACKGROUND_THROTTLE_TARGETS: &[&str] = &[
    "chrome",
    "msedge",
    "firefox",
    "discord",
    "spotify",
    "onedrive",
    "dropbox",
    "steamwebhelper",
    "epicgameslauncher",
];

/// RAII Guard: guarantees `timeEndPeriod(1)` ALWAYS fires when dropped or unwound on panic.
pub struct TimerResolutionGuard {
    period_ms: u32,
}

impl TimerResolutionGuard {
    pub fn acquire(period_ms: u32) -> Self {
        unsafe {
            let _ = timeBeginPeriod(period_ms);
        }
        Self { period_ms }
    }
}

impl Drop for TimerResolutionGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = timeEndPeriod(self.period_ms);
        }
    }
}

struct ThrottledProcBackup {
    creation_time: u64,
    original_priority: u32,
}

struct ActiveInterventions {
    timer_guard: Option<TimerResolutionGuard>,
    previous_power_guid: Option<String>,
    throttled_bg: HashMap<u32, ThrottledProcBackup>,
}

impl ActiveInterventions {
    fn revert(&mut self, logs: &SharedLogs) {
        self.timer_guard = None; // Drop releases timeEndPeriod(1)!

        // Always restore power scheme if changed — even if user toggled checkbox mid-game
        if let Some(prev) = self.previous_power_guid.take() {
            if let Some(g) = parse_guid(&prev) {
                unsafe {
                    let _ = PowerSetActiveScheme(None, Some(&g));
                }
                push_log(logs, LogLevel::Info, format!("[GameMode] Restored power plan {prev}"));
            }
        }

        // Restore background process priorities safely with PID creation-time verification
        for (pid, bkp) in self.throttled_bg.drain() {
            unsafe {
                if let Ok(handle) =
                    OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_INFORMATION, false, pid)
                {
                    if read_creation_time(handle) == Some(bkp.creation_time) {
                        let _ = SetPriorityClass(handle, PROCESS_CREATION_FLAGS(bkp.original_priority));
                    }
                    let _ = CloseHandle(handle);
                }
            }
        }
    }
}

pub struct GameModeController {
    config: Arc<Mutex<GameModeConfig>>,
    status: Arc<Mutex<GameModeStatus>>,
    running: Arc<AtomicBool>,
    p_core_mask: Arc<Mutex<u64>>,
    thread_handle: Mutex<Option<JoinHandle<()>>>,
    logs: SharedLogs,
}

impl GameModeController {
    pub fn new(logs: SharedLogs, initial_mask: u64) -> Self {
        let cfg = load_saved_config();
        Self {
            config: Arc::new(Mutex::new(cfg)),
            status: Arc::new(Mutex::new(GameModeStatus::default())),
            running: Arc::new(AtomicBool::new(false)),
            p_core_mask: Arc::new(Mutex::new(initial_mask)),
            thread_handle: Mutex::new(None),
            logs,
        }
    }

    pub fn update_affinity_mask(&self, mask: u64) {
        if let Ok(mut g) = self.p_core_mask.lock() {
            *g = mask;
        }
    }

    pub fn get_config(&self) -> GameModeConfig {
        self.config.lock().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn update_config(&self, cfg: GameModeConfig) {
        save_config(&cfg);
        if let Ok(mut g) = self.config.lock() {
            *g = cfg;
        }
    }

    pub fn add_custom_target(&self, raw_name: &str) -> Result<String, String> {
        let clean = normalize_proc_name(raw_name);
        if clean.is_empty() {
            return Err("Process name cannot be empty".to_string());
        }
        let mut g = self.config.lock().map_err(|_| "Lock error".to_string())?;
        if !g.custom_targets.iter().any(|x| x.eq_ignore_ascii_case(&clean)) {
            g.custom_targets.push(clean.clone());
            save_config(&g);
        }
        Ok(clean)
    }

    pub fn remove_custom_target(&self, name: &str) {
        if let Ok(mut g) = self.config.lock() {
            g.custom_targets.retain(|x| !x.eq_ignore_ascii_case(name));
            save_config(&g);
        }
    }

    pub fn status(&self) -> GameModeStatus {
        let mut s = self.status.lock().map(|g| g.clone()).unwrap_or_default();
        s.current_timer_ms = query_timer_resolution_ms().2;
        s
    }

    #[allow(dead_code)]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn start(&self) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }
        push_log(
            &self.logs,
            LogLevel::Info,
            "[GameMode] Monitoring started (Vanguard/EAC safe; RAII timer guard armed).",
        );

        let running = Arc::clone(&self.running);
        let config = Arc::clone(&self.config);
        let status = Arc::clone(&self.status);
        let mask_arc = Arc::clone(&self.p_core_mask);
        let logs = Arc::clone(&self.logs);

        let handle = std::thread::spawn(move || {
            let mut active: Option<(u32, u64, String, bool, ActiveInterventions)> = None;

            if let Ok(mut st) = status.lock() {
                st.daemon_running = true;
                st.summary = "Monitoring for games & emulators...".to_string();
            }

            while running.load(Ordering::Relaxed) {
                let cfg = config.lock().map(|g| g.clone()).unwrap_or_default();
                let mask = mask_arc.lock().map(|g| *g).unwrap_or(0);
                let procs = snapshot_processes();

                // Verify currently active game still matches PID + creation timestamp
                let still_running = if let Some((pid, ctime, _, _, _)) = &active {
                    procs.iter().any(|p| p.pid == *pid) && verify_pid_creation_time(*pid, *ctime)
                } else {
                    false
                };

                if active.is_some() && !still_running {
                    if let Some((_, _, name, _, mut interv)) = active.take() {
                        interv.revert(&logs);
                        push_log(
                            &logs,
                            LogLevel::Info,
                            format!("[GameMode] '{name}' exited — reverted all session overrides."),
                        );
                    }
                    if let Ok(mut st) = status.lock() {
                        st.daemon_running = true;
                        st.active_game_name = None;
                        st.active_game_pid = None;
                        st.is_anticheat_protected = false;
                        st.interventions.clear();
                        st.summary = "Standby — waiting for game launch...".to_string();
                    }
                }

                if active.is_none() {
                    if let Some((proc_entry, anticheat)) = find_matching_target(&procs, &cfg) {
                        let ctime = query_pid_creation_time(proc_entry.pid).unwrap_or(0);
                        let mut applied_labels = HashSet::new();
                        let mut interv = ActiveInterventions {
                            timer_guard: None,
                            previous_power_guid: None,
                            throttled_bg: HashMap::new(),
                        };

                        if !anticheat {
                            if cfg.enable_p_core_pinning && mask > 0 {
                                if apply_affinity_and_power_throttling(proc_entry.pid, mask) {
                                    applied_labels.insert("P-Core Affinity".to_string());
                                }
                            }
                            if cfg.enable_high_priority && apply_high_priority(proc_entry.pid) {
                                applied_labels.insert("High Priority".to_string());
                            }
                        } else {
                            applied_labels.insert("Anti-Cheat Safe (No Handle Probe)".to_string());
                        }

                        if cfg.enable_1ms_timer {
                            interv.timer_guard = Some(TimerResolutionGuard::acquire(1));
                            applied_labels.insert("1.0ms Timer (RAII)".to_string());
                        }

                        if cfg.enable_power_scheme {
                            if let Some((prev_guid, _)) = get_active_power_scheme() {
                                if let Ok(perf_guid) = ensure_or_find_performance_scheme_guid() {
                                    let perf_str = format_guid(&perf_guid);
                                    if !prev_guid.eq_ignore_ascii_case(&perf_str) {
                                        let err =
                                            unsafe { PowerSetActiveScheme(None, Some(&perf_guid)) };
                                        if err.0 == 0 {
                                            interv.previous_power_guid = Some(prev_guid);
                                            applied_labels.insert("Performance Power".to_string());
                                        }
                                    }
                                }
                            }
                        }

                        if cfg.enable_background_throttle {
                            throttle_background_apps(&procs, &mut interv.throttled_bg);
                            if !interv.throttled_bg.is_empty() {
                                applied_labels.insert(format!(
                                    "Throttled {} BG Apps",
                                    interv.throttled_bg.len()
                                ));
                            }
                        }

                        let mut ordered: Vec<String> = applied_labels.iter().cloned().collect();
                        ordered.sort();
                        let summary = ordered.join(" • ");

                        push_log(
                            &logs,
                            LogLevel::Ok,
                            format!(
                                "[GameMode] Detected {} (PID {}) -> {}",
                                proc_entry.name, proc_entry.pid, summary
                            ),
                        );

                        if let Ok(mut st) = status.lock() {
                            st.daemon_running = true;
                            st.active_game_name = Some(proc_entry.name.clone());
                            st.active_game_pid = Some(proc_entry.pid);
                            st.is_anticheat_protected = anticheat;
                            st.affinity_mask_hex = if anticheat || mask == 0 {
                                None
                            } else {
                                Some(format!("0x{mask:X}"))
                            };
                            st.interventions = applied_labels;
                            st.summary = summary;
                        }

                        active = Some((proc_entry.pid, ctime, proc_entry.name.clone(), anticheat, interv));
                    }
                }

                for _ in 0..15 {
                    if !running.load(Ordering::Relaxed) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }

            if let Some((_, _, name, _, mut interv)) = active.take() {
                interv.revert(&logs);
                push_log(
                    &logs,
                    LogLevel::Info,
                    format!("[GameMode] Stopped while tracking '{name}' — clean revert done."),
                );
            }

            if let Ok(mut st) = status.lock() {
                *st = GameModeStatus::default();
            }
        });

        if let Ok(mut g) = self.thread_handle.lock() {
            *g = Some(handle);
        }
    }

    pub fn stop(&self) {
        if !self.running.swap(false, Ordering::SeqCst) {
            return;
        }
        if let Ok(mut g) = self.thread_handle.lock() {
            if let Some(h) = g.take() {
                let _ = h.join();
            }
        }
        push_log(&self.logs, LogLevel::Info, "[GameMode] Daemon stopped cleanly.");
    }
}

impl Drop for GameModeController {
    fn drop(&mut self) {
        self.stop();
    }
}

struct SnapProc {
    pid: u32,
    name: String,
    stem_lower: String,
}

fn snapshot_processes() -> Vec<SnapProc> {
    let mut list = Vec::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return list;
        };
        if snap == INVALID_HANDLE_VALUE || snap.is_invalid() {
            return list;
        }
        let mut pe = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snap, &mut pe).is_ok() {
            loop {
                let end = pe
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(pe.szExeFile.len());
                let name = String::from_utf16_lossy(&pe.szExeFile[..end]);
                let stem_lower = normalize_proc_name(&name);
                if pe.th32ProcessID > 4 && !stem_lower.is_empty() {
                    list.push(SnapProc {
                        pid: pe.th32ProcessID,
                        name,
                        stem_lower,
                    });
                }
                if Process32NextW(snap, &mut pe).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    list
}

fn find_matching_target<'a>(
    procs: &'a [SnapProc],
    cfg: &GameModeConfig,
) -> Option<(&'a SnapProc, bool)> {
    // Prefer standard games/emulators or anti-cheat games; mark anticheat flag accurately
    for p in procs {
        if ANTICHEAT_PROTECTED.iter().any(|&a| p.stem_lower == a) {
            return Some((p, true));
        }
        if STANDARD_TARGETS.iter().any(|&t| p.stem_lower == t) {
            return Some((p, false));
        }
        if cfg
            .custom_targets
            .iter()
            .any(|c| c.eq_ignore_ascii_case(&p.stem_lower))
        {
            let ac = ANTICHEAT_PROTECTED.iter().any(|&a| p.stem_lower == a);
            return Some((p, ac));
        }
    }
    None
}

pub fn normalize_proc_name(raw: &str) -> String {
    let s = raw.trim().to_ascii_lowercase();
    s.strip_suffix(".exe").unwrap_or(&s).to_string()
}

fn query_pid_creation_time(pid: u32) -> Option<u64> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let t = read_creation_time(h);
        let _ = CloseHandle(h);
        t
    }
}

fn verify_pid_creation_time(pid: u32, expected: u64) -> bool {
    if expected == 0 {
        return true;
    }
    query_pid_creation_time(pid) == Some(expected)
}

unsafe fn read_creation_time(handle: windows::Win32::Foundation::HANDLE) -> Option<u64> {
    let (mut c, mut e, mut k, mut u) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    if GetProcessTimes(handle, &mut c, &mut e, &mut k, &mut u).is_ok() {
        Some(((c.dwHighDateTime as u64) << 32) | (c.dwLowDateTime as u64))
    } else {
        None
    }
}

fn apply_affinity_and_power_throttling(pid: u32, mask: u64) -> bool {
    unsafe {
        let Ok(handle) =
            OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_INFORMATION, false, pid)
        else {
            return false;
        };
        let aff_ok = SetProcessAffinityMask(handle, mask as usize).is_ok();
        let state = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            StateMask: 0,
        };
        let _ = SetProcessInformation(
            handle,
            ProcessPowerThrottling,
            &state as *const _ as *const _,
            std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
        );
        let _ = CloseHandle(handle);
        aff_ok
    }
}

fn apply_high_priority(pid: u32) -> bool {
    unsafe {
        let Ok(handle) =
            OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_INFORMATION, false, pid)
        else {
            return false;
        };
        let ok = SetPriorityClass(handle, HIGH_PRIORITY_CLASS).is_ok();
        let _ = CloseHandle(handle);
        ok
    }
}

fn throttle_background_apps(procs: &[SnapProc], map: &mut HashMap<u32, ThrottledProcBackup>) {
    for p in procs {
        if !BACKGROUND_THROTTLE_TARGETS.iter().any(|&b| p.stem_lower == b) {
            continue;
        }
        unsafe {
            let Ok(handle) =
                OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_INFORMATION, false, p.pid)
            else {
                continue;
            };
            let orig_prio = GetPriorityClass(handle);
            let ctime = read_creation_time(handle);
            if orig_prio != 0 && orig_prio != BELOW_NORMAL_PRIORITY_CLASS.0 {
                if let Some(ct) = ctime {
                    if SetPriorityClass(handle, BELOW_NORMAL_PRIORITY_CLASS).is_ok() {
                        map.insert(
                            p.pid,
                            ThrottledProcBackup {
                                creation_time: ct,
                                original_priority: orig_prio,
                            },
                        );
                    }
                }
            }
            let _ = CloseHandle(handle);
        }
    }
}

fn load_saved_config() -> GameModeConfig {
    let p = gamemode_config_path();
    std::fs::read_to_string(p)
        .ok()
        .and_then(|s| serde_json::from_str::<GameModeConfig>(&s).ok())
        .unwrap_or_default()
}

fn save_config(cfg: &GameModeConfig) {
    crate::engine::ensure_app_dirs();
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        let _ = std::fs::write(gamemode_config_path(), json);
    }
}

#[cfg(test)]
mod tests {
    use crate::system::cpu::compute_group0_all_mask;

    #[test]
    fn cpu_affinity_mask_64core_safety_tests() {
        assert_eq!(compute_group0_all_mask(0), 1);
        assert_eq!(compute_group0_all_mask(16), 0xFFFF);
        assert_eq!(compute_group0_all_mask(20), 0xF_FFFF);
        assert_eq!(compute_group0_all_mask(64), u64::MAX);
        assert_eq!(compute_group0_all_mask(128), u64::MAX);
    }
}
