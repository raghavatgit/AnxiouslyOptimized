use crate::engine::gamemode::GameModeController;
use crate::engine::maintenance::{HibernationStatus, StandbyPurgeResult};
use crate::engine::updater::APP_VERSION;
use crate::engine::{
    cleaner, debloat, emulator, format_bytes, installer, is_process_elevated, load_settings,
    maintenance, network, push_log, runtimes, safety, save_settings, tweaks, uninstaller, updater,
    valorant,
};
use crate::models::{
    BloatAppItem, CleanerCategory, DnsProviderResult, EmulatorTopology, GameModeStatus,
    InstalledAppEntry, LeftoverItem, LogLevel, MemReductLiveStats, RestorePointInfo,
    RiotRegionPing, RuntimePackageItem, SystemReport, ToastKind, TransactionJournal, TweakViewItem,
    UpdateCheckResult, UpdateStatus, ValorantTopology, VssHealth,
};
use crate::ui::components::{
    danger_button, emerald_button, icon_button, paint_icon, search_bar, status_badge,
    subtle_button, NavIcon,
};
use crate::ui::{tabs, theme, AppAction, AppsSubView, CleanSubView, GamingSubView, TabId, UiState};
use chrono::Local;
use egui::{Color32, CornerRadius, Margin, Pos2, Rect, RichText, Stroke, Vec2};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

enum BgMsg {
    TelemetryTick {
        cpu_pct: f32,
        ram_used: u64,
        ram_total: u64,
        mem_stats: MemReductLiveStats,
        gm_status: GameModeStatus,
    },
    InitialBootReady {
        report: SystemReport,
        tweaks: Vec<TweakViewItem>,
        cleaner_cats: Vec<CleanerCategory>,
        hibernation: HibernationStatus,
        runtimes: Vec<RuntimePackageItem>,
        installed_apps: Vec<InstalledAppEntry>,
        valorant: ValorantTopology,
        emulators: EmulatorTopology,
        journals: Vec<TransactionJournal>,
        vss: VssHealth,
        rps: Vec<RestorePointInfo>,
    },
    SystemReportReady(SystemReport),
    TweaksAudited(Vec<TweakViewItem>),
    CleanerScanned(Vec<CleanerCategory>),
    CleanerPurged {
        cats: Vec<CleanerCategory>,
        freed: u64,
        deleted: usize,
        failed: usize,
    },
    StandbyPurged(Result<StandbyPurgeResult, String>),
    HibernationUpdated(Result<HibernationStatus, String>),
    StreamLine(String),
    StreamDone {
        label: String,
        result: Result<String, String>,
        refresh_runtimes: bool,
    },
    RuntimesReady(Vec<RuntimePackageItem>),
    InstalledAppsReady(Vec<InstalledAppEntry>),
    LeftoversScanned {
        app: InstalledAppEntry,
        items: Vec<LeftoverItem>,
        toast: Option<String>,
    },
    LeftoversDeleted {
        deleted: usize,
        failed: usize,
        freed: u64,
        apps: Vec<InstalledAppEntry>,
    },
    BloatReady(Result<Vec<BloatAppItem>, String>),
    BloatRemoved {
        remaining: Result<Vec<BloatAppItem>, String>,
        summary: String,
    },
    DnsBenchmarked(Vec<DnsProviderResult>),
    ValorantReady(ValorantTopology),
    RiotPingReady(Vec<RiotRegionPing>),
    EmulatorReady(EmulatorTopology),
    SafetyReady {
        journals: Vec<TransactionJournal>,
        vss: VssHealth,
        rps: Vec<RestorePointInfo>,
    },
    UpdateCheckFinished(Result<UpdateCheckResult, String>),
    InstallFinished(Result<String, String>),
    UninstallFinished(Result<String, String>),
    OperationDone {
        kind: ToastKind,
        msg: String,
        refresh_tweaks: bool,
        refresh_system: bool,
        refresh_valorant: bool,
        refresh_emulators: bool,
        refresh_safety: bool,
        explorer_prompt: bool,
    },
}

pub struct OptimizerApp {
    pub state: UiState,
    tx: mpsc::Sender<BgMsg>,
    rx: mpsc::Receiver<BgMsg>,
    telemetry_running: Arc<AtomicBool>,
    last_auto_reduct: Instant,
}

impl OptimizerApp {
    pub fn new(cc: &eframe::CreationContext<'_>, logo_rgba: Option<(Vec<u8>, u32, u32)>) -> Self {
        theme::apply_theme(&cc.egui_ctx);

        let logo_tex = logo_rgba.map(|(rgba, w, h)| {
            let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            cc.egui_ctx
                .load_texture("obr_logo", img, egui::TextureOptions::LINEAR)
        });

        let settings = load_settings();
        theme::apply_scale_metrics(&cc.egui_ctx, settings.ui_scale);
        let is_elevated = is_process_elevated();
        let is_installed_on_pc = installer::is_app_installed();
        let shared_logs = Arc::new(std::sync::Mutex::new(Vec::new()));
        let game_mode_controller = Arc::new(GameModeController::new(Arc::clone(&shared_logs), 0));
        let gamemode_cfg = game_mode_controller.get_config();
        let gamemode_status = game_mode_controller.status();

        push_log(
            &shared_logs,
            LogLevel::Info,
            format!(
                "Optimizer by Raghav v{APP_VERSION} started (Privilege: {}, Mode: {}). Scanning hardware...",
                if is_elevated {
                    "Administrator"
                } else {
                    "Standard User"
                },
                if is_installed_on_pc {
                    "Installed"
                } else {
                    "Portable"
                }
            ),
        );

        let (tx, rx) = mpsc::channel::<BgMsg>();
        let telemetry_running = Arc::new(AtomicBool::new(true));

        // 1-second live telemetry worker thread
        {
            let tx_t = tx.clone();
            let run_flag = Arc::clone(&telemetry_running);
            let gmc = Arc::clone(&game_mode_controller);
            let ctx = cc.egui_ctx.clone();
            std::thread::spawn(move || {
                let mut sys = sysinfo::System::new_with_specifics(
                    sysinfo::RefreshKind::nothing()
                        .with_cpu(sysinfo::CpuRefreshKind::everything())
                        .with_memory(sysinfo::MemoryRefreshKind::everything()),
                );
                std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
                while run_flag.load(Ordering::Relaxed) {
                    sys.refresh_cpu_usage();
                    sys.refresh_memory();
                    let cpu_pct = sys.global_cpu_usage();
                    let mem_stats = maintenance::query_mem_reduct_stats();
                    let ram_total = if mem_stats.phys_total_bytes > 0 {
                        mem_stats.phys_total_bytes
                    } else {
                        sys.total_memory()
                    };
                    let ram_used = if mem_stats.phys_total_bytes > 0 {
                        mem_stats.phys_used_bytes
                    } else {
                        sys.used_memory()
                    };
                    let gm_status = gmc.status();
                    let _ = tx_t.send(BgMsg::TelemetryTick {
                        cpu_pct,
                        ram_used,
                        ram_total,
                        mem_stats,
                        gm_status,
                    });
                    ctx.request_repaint();
                    for _ in 0..10 {
                        if !run_flag.load(Ordering::Relaxed) {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
            });
        }

        let state = UiState {
            active_tab: TabId::Dashboard,
            clean_subview: CleanSubView::Memory,
            apps_subview: AppsSubView::Installed,
            gaming_subview: GamingSubView::Engine,
            show_command_palette: false,
            command_query: String::new(),
            is_elevated,
            is_installed_on_pc,
            update_status: UpdateStatus::Idle,
            show_update_modal: false,
            settings: settings.clone(),
            logo_tex,
            system_report: SystemReport::default(),
            scanning_system: true,
            cpu_history: VecDeque::with_capacity(64),
            ram_history: VecDeque::with_capacity(64),
            live_cpu_pct: 0.0,
            live_ram_used_bytes: 0,
            live_ram_total_bytes: 0,
            mem_stats: maintenance::query_mem_reduct_stats(),
            tweaks: tweaks::catalog_for_os(true),
            tweaks_search: String::new(),
            tweaks_category: None,
            explorer_restart_needed: false,
            cleaner_cats: cleaner::get_default_categories(settings.temp_cleaner_min_age_hours),
            hibernation: maintenance::get_hibernation_status(),
            last_standby_purge: None,
            stream_lines: Vec::new(),
            stream_running: false,
            stream_label: String::new(),
            runtimes: Vec::new(),
            installed_apps: Vec::new(),
            uninstaller_search: String::new(),
            uninstaller_filter_kind: None,
            uninstaller_include_uwp: false,
            uninstaller_sort_by_size: true,
            active_leftover_app: None,
            leftover_items: Vec::new(),
            scanning_apps: true,
            bloat_items: Vec::new(),
            bloat_search: String::new(),
            dns_providers: network::default_dns_providers(),
            dns_custom_primary: "1.1.1.1".to_string(),
            dns_custom_secondary: "1.0.0.1".to_string(),
            selected_adapter_idx: 0,
            gamemode_cfg,
            gamemode_status,
            gamemode_custom_input: String::new(),
            valorant_topo: ValorantTopology::default(),
            riot_pings: Vec::new(),
            emulator_topo: EmulatorTopology::default(),
            journals: Vec::new(),
            vss_health: VssHealth::default(),
            restore_points: Vec::new(),
            restore_desc_input: "Before Optimization".to_string(),
            shared_logs,
            logs_snapshot: Vec::new(),
            last_log_count: 0,
            toasts: Vec::new(),
            pending_confirm: None,
            busy_task: Some("Scanning whole-system hardware, tweaks & installed apps...".to_string()),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            game_mode_controller,
        };

        let app = Self {
            state,
            tx,
            rx,
            telemetry_running,
            last_auto_reduct: Instant::now(),
        };
        app.spawn_initial_boot(&cc.egui_ctx);
        app
    }

    fn spawn_initial_boot(&self, ctx: &egui::Context) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let gmc = Arc::clone(&self.state.game_mode_controller);
        let temp_age = self.state.settings.temp_cleaner_min_age_hours;
        std::thread::spawn(move || {
            let report = SystemReport::collect();
            let mask = if report.cpu.x3d_ccd0_mask != 0 {
                report.cpu.x3d_ccd0_mask
            } else {
                report.cpu.p_core_mask
            };
            gmc.update_affinity_mask(mask);

            let mut tw = tweaks::catalog_for_os(report.platform.is_win11);
            tweaks::audit_tweaks(&mut tw);

            let mut cleaner_cats = cleaner::get_default_categories(temp_age);
            cleaner::scan_categories(&mut cleaner_cats, temp_age);

            let hibernation = maintenance::get_hibernation_status();
            let runtimes_list = runtimes::get_runtime_catalog();
            let mut installed_apps = uninstaller::scan_all_installed_apps(false);
            installed_apps.sort_by(|a, b| b.estimated_size_bytes.cmp(&a.estimated_size_bytes));
            let valorant = valorant::detect_topology();
            let emulators = emulator::detect_topology();
            let journals = safety::load_all_journals();
            let vss = safety::check_vss_health();
            let rps = safety::list_restore_points();

            let _ = tx.send(BgMsg::InitialBootReady {
                report,
                tweaks: tw,
                cleaner_cats,
                hibernation,
                runtimes: runtimes_list,
                installed_apps,
                valorant,
                emulators,
                journals,
                vss,
                rps,
            });
            ctx.request_repaint();
        });
    }

    fn sort_apps_in_place(&mut self) {
        if self.state.uninstaller_sort_by_size {
            self.state
                .installed_apps
                .sort_by(|a, b| b.estimated_size_bytes.cmp(&a.estimated_size_bytes));
        } else {
            self.state.installed_apps.sort_by(|a, b| {
                a.display_name
                    .to_ascii_lowercase()
                    .cmp(&b.display_name.to_ascii_lowercase())
            });
        }
    }

    fn process_bg_messages(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        let mut trigger_auto_reduct = false;

        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                BgMsg::TelemetryTick {
                    cpu_pct,
                    ram_used,
                    ram_total,
                    mem_stats,
                    gm_status,
                } => {
                    self.state.live_cpu_pct = cpu_pct;
                    self.state.live_ram_used_bytes = ram_used;
                    self.state.live_ram_total_bytes = ram_total;
                    self.state.mem_stats = mem_stats;
                    self.state.gamemode_status = gm_status;
                    self.state.cpu_history.push_back(cpu_pct);
                    if self.state.cpu_history.len() > 60 {
                        self.state.cpu_history.pop_front();
                    }
                    let ram_pct = if ram_total > 0 {
                        (ram_used as f64 / ram_total as f64 * 100.0) as f32
                    } else {
                        0.0
                    };
                    self.state.ram_history.push_back(ram_pct);
                    if self.state.ram_history.len() > 60 {
                        self.state.ram_history.pop_front();
                    }

                    let mr = &self.state.settings.mem_reduct;
                    if mr.auto_reduct_enabled
                        && self.state.busy_task.is_none()
                        && self.last_auto_reduct.elapsed() >= Duration::from_secs(60)
                    {
                        let over_threshold =
                            self.state.mem_stats.phys_load_pct >= mr.auto_reduct_threshold_pct;
                        let over_interval = mr.auto_reduct_interval_mins > 0
                            && self.last_auto_reduct.elapsed()
                                >= Duration::from_secs((mr.auto_reduct_interval_mins as u64) * 60);
                        if over_threshold || over_interval {
                            self.last_auto_reduct = Instant::now();
                            trigger_auto_reduct = true;
                        }
                    }
                }
                BgMsg::InitialBootReady {
                    report,
                    tweaks,
                    cleaner_cats,
                    hibernation,
                    runtimes,
                    installed_apps,
                    valorant,
                    emulators,
                    journals,
                    vss,
                    rps,
                } => {
                    push_log(
                        &self.state.shared_logs,
                        LogLevel::Ok,
                        format!(
                            "System scan complete: {} | {} GPU(s) | {} RAM | {} tweaks | {} installed programs",
                            report.cpu.brand,
                            report.gpus.len(),
                            format_bytes(report.memory.total_phys_bytes),
                            tweaks.len(),
                            installed_apps.len()
                        ),
                    );
                    self.state.system_report = report;
                    self.state.scanning_system = false;
                    self.state.scanning_apps = false;
                    self.state.busy_task = None;
                    self.state.tweaks = tweaks;
                    self.state.cleaner_cats = cleaner_cats;
                    self.state.hibernation = hibernation;
                    self.state.runtimes = runtimes;
                    self.state.installed_apps = installed_apps;
                    self.state.valorant_topo = valorant;
                    self.state.emulator_topo = emulators;
                    self.state.journals = journals;
                    self.state.vss_health = vss;
                    self.state.restore_points = rps;
                }
                BgMsg::SystemReportReady(report) => {
                    let mask = if report.cpu.x3d_ccd0_mask != 0 {
                        report.cpu.x3d_ccd0_mask
                    } else {
                        report.cpu.p_core_mask
                    };
                    self.state.game_mode_controller.update_affinity_mask(mask);
                    self.state.system_report = report;
                    self.state.scanning_system = false;
                    self.state.busy_task = None;
                    self.state
                        .push_toast(now, ToastKind::Success, "System hardware rescanned");
                }
                BgMsg::TweaksAudited(tw) => {
                    self.state.tweaks = tw;
                    self.state.busy_task = None;
                }
                BgMsg::CleanerScanned(cats) => {
                    let total: u64 = cats
                        .iter()
                        .filter(|c| c.is_selected)
                        .map(|c| c.scanned_bytes)
                        .sum();
                    self.state.cleaner_cats = cats;
                    self.state.busy_task = None;
                    self.state.push_toast(
                        now,
                        ToastKind::Info,
                        format!("PC Cleaner scan complete: {} reclaimable", format_bytes(total)),
                    );
                }
                BgMsg::CleanerPurged {
                    cats,
                    freed,
                    deleted,
                    failed,
                } => {
                    self.state.cleaner_cats = cats;
                    self.state.busy_task = None;
                    let msg = format!(
                        "Cleaned {deleted} file(s), freed {} ({failed} in-use/skipped)",
                        format_bytes(freed)
                    );
                    push_log(&self.state.shared_logs, LogLevel::Ok, &msg);
                    self.state.push_toast(now, ToastKind::Success, msg);
                }
                BgMsg::StandbyPurged(res) => {
                    self.state.busy_task = None;
                    self.state.mem_stats = maintenance::query_mem_reduct_stats();
                    match res {
                        Ok(r) => {
                            let gain = r.avail_after_bytes.saturating_sub(r.avail_before_bytes);
                            let used_drop = r.used_before_bytes.saturating_sub(r.used_after_bytes);
                            let freed = gain.max(used_drop);
                            let msg = format!(
                                "Mem Reduct cleaned {}: freed {} (Cache {} -> {})",
                                r.regions_cleaned.join(" + "),
                                format_bytes(freed),
                                format_bytes(r.cache_before_bytes),
                                format_bytes(r.cache_after_bytes)
                            );
                            self.state.last_standby_purge = Some(r);
                            push_log(&self.state.shared_logs, LogLevel::Ok, &msg);
                            self.state.push_toast(now, ToastKind::Success, msg);
                        }
                        Err(e) => {
                            push_log(&self.state.shared_logs, LogLevel::Error, &e);
                            self.state.push_toast(now, ToastKind::Error, e);
                        }
                    }
                }
                BgMsg::HibernationUpdated(res) => {
                    self.state.busy_task = None;
                    match res {
                        Ok(h) => {
                            let msg = if h.enabled {
                                format!("Hibernation enabled ({})", format_bytes(h.size_bytes))
                            } else {
                                "Hibernation disabled & hiberfil.sys removed".to_string()
                            };
                            self.state.hibernation = h;
                            push_log(&self.state.shared_logs, LogLevel::Ok, &msg);
                            self.state.push_toast(now, ToastKind::Success, msg);
                        }
                        Err(e) => {
                            push_log(&self.state.shared_logs, LogLevel::Error, &e);
                            self.state.push_toast(now, ToastKind::Error, e);
                        }
                    }
                }
                BgMsg::StreamLine(line) => {
                    self.state.stream_lines.push(line);
                    if self.state.stream_lines.len() > 600 {
                        let cut = self.state.stream_lines.len() - 600;
                        self.state.stream_lines.drain(0..cut);
                    }
                }
                BgMsg::StreamDone {
                    label,
                    result,
                    refresh_runtimes,
                } => {
                    self.state.stream_running = false;
                    self.state.busy_task = None;
                    match result {
                        Ok(msg) => {
                            self.state.stream_lines.push(format!("[DONE] {label}: {msg}"));
                            push_log(&self.state.shared_logs, LogLevel::Ok, &msg);
                            self.state.push_toast(now, ToastKind::Success, msg);
                        }
                        Err(e) => {
                            self.state.stream_lines.push(format!("[ERROR] {label}: {e}"));
                            push_log(&self.state.shared_logs, LogLevel::Error, &e);
                            self.state.push_toast(now, ToastKind::Error, e);
                        }
                    }
                    if refresh_runtimes {
                        self.handle_action(AppAction::RefreshRuntimes, ctx);
                    }
                }
                BgMsg::RuntimesReady(rt) => {
                    self.state.runtimes = rt;
                    self.state.busy_task = None;
                }
                BgMsg::InstalledAppsReady(apps) => {
                    self.state.installed_apps = apps;
                    self.sort_apps_in_place();
                    self.state.scanning_apps = false;
                    self.state.busy_task = None;
                }
                BgMsg::LeftoversScanned { app, items, toast } => {
                    self.state.busy_task = None;
                    let cnt = items.len();
                    let name = app.display_name.clone();
                    self.state.active_leftover_app = Some(app);
                    self.state.leftover_items = items;
                    let msg = toast.unwrap_or_else(|| {
                        format!("Geek Scanner: found {cnt} leftover trace(s) for '{name}'")
                    });
                    push_log(&self.state.shared_logs, LogLevel::Info, &msg);
                    self.state.push_toast(now, ToastKind::Info, msg);
                }
                BgMsg::LeftoversDeleted {
                    deleted,
                    failed,
                    freed,
                    apps,
                } => {
                    self.state.busy_task = None;
                    self.state.installed_apps = apps;
                    self.sort_apps_in_place();
                    self.state.active_leftover_app = None;
                    self.state.leftover_items.clear();
                    let msg = format!(
                        "Geek Force-Cleaner: removed {deleted} item(s), freed {} ({failed} skipped)",
                        format_bytes(freed)
                    );
                    push_log(&self.state.shared_logs, LogLevel::Ok, &msg);
                    self.state.push_toast(now, ToastKind::Success, msg);
                }
                BgMsg::BloatReady(res) => {
                    self.state.busy_task = None;
                    match res {
                        Ok(list) => {
                            let msg = format!(
                                "Scanned installed apps: {} bloat candidate(s) found",
                                list.len()
                            );
                            self.state.bloat_items = list;
                            push_log(&self.state.shared_logs, LogLevel::Info, &msg);
                            self.state.push_toast(now, ToastKind::Info, msg);
                        }
                        Err(e) => {
                            push_log(&self.state.shared_logs, LogLevel::Error, &e);
                            self.state.push_toast(now, ToastKind::Error, e);
                        }
                    }
                }
                BgMsg::BloatRemoved { remaining, summary } => {
                    self.state.busy_task = None;
                    if let Ok(list) = remaining {
                        self.state.bloat_items = list;
                    }
                    push_log(&self.state.shared_logs, LogLevel::Ok, &summary);
                    self.state.push_toast(now, ToastKind::Success, summary);
                }
                BgMsg::DnsBenchmarked(list) => {
                    self.state.busy_task = None;
                    let fastest_msg = list
                        .iter()
                        .find(|p| p.is_fastest)
                        .map(|p| format!("Fastest DNS: {}", p.name))
                        .unwrap_or_else(|| "DNS benchmark finished".to_string());
                    self.state.dns_providers = list;
                    push_log(&self.state.shared_logs, LogLevel::Ok, &fastest_msg);
                    self.state.push_toast(now, ToastKind::Success, fastest_msg);
                }
                BgMsg::ValorantReady(topo) => {
                    self.state.valorant_topo = topo;
                    self.state.busy_task = None;
                }
                BgMsg::RiotPingReady(pings) => {
                    self.state.riot_pings = pings;
                    self.state.busy_task = None;
                    self.state
                        .push_toast(now, ToastKind::Info, "Riot regional latencies measured");
                }
                BgMsg::EmulatorReady(topo) => {
                    self.state.emulator_topo = topo;
                    self.state.busy_task = None;
                }
                BgMsg::SafetyReady { journals, vss, rps } => {
                    self.state.journals = journals;
                    self.state.vss_health = vss;
                    self.state.restore_points = rps;
                    self.state.busy_task = None;
                }
                BgMsg::UpdateCheckFinished(res) => {
                    self.state.busy_task = None;
                    match res {
                        Ok(info) => {
                            if info.is_newer {
                                let msg = format!(
                                    "New version available: v{} (current v{})",
                                    info.latest_version, info.current_version
                                );
                                self.state.update_status = UpdateStatus::Available(info);
                                self.state.show_update_modal = true;
                                push_log(&self.state.shared_logs, LogLevel::Ok, &msg);
                                self.state.push_toast(now, ToastKind::Info, msg);
                            } else {
                                let msg = format!(
                                    "You are running the latest version (v{})",
                                    info.current_version
                                );
                                self.state.update_status = UpdateStatus::UpToDate {
                                    version: info.current_version,
                                };
                                push_log(&self.state.shared_logs, LogLevel::Ok, &msg);
                                self.state.push_toast(now, ToastKind::Success, msg);
                            }
                        }
                        Err(e) => {
                            self.state.update_status = UpdateStatus::Error(e.clone());
                            push_log(&self.state.shared_logs, LogLevel::Warn, &e);
                            self.state.push_toast(now, ToastKind::Warning, e);
                        }
                    }
                }
                BgMsg::InstallFinished(res) => {
                    self.state.busy_task = None;
                    self.state.is_installed_on_pc = installer::is_app_installed();
                    match res {
                        Ok(msg) => {
                            push_log(&self.state.shared_logs, LogLevel::Ok, &msg);
                            self.state.push_toast(now, ToastKind::Success, msg);
                        }
                        Err(e) => {
                            push_log(&self.state.shared_logs, LogLevel::Error, &e);
                            self.state.push_toast(now, ToastKind::Error, e);
                        }
                    }
                }
                BgMsg::UninstallFinished(res) => {
                    self.state.busy_task = None;
                    self.state.is_installed_on_pc = installer::is_app_installed();
                    match res {
                        Ok(msg) => {
                            push_log(&self.state.shared_logs, LogLevel::Info, &msg);
                            self.state.push_toast(now, ToastKind::Info, msg);
                        }
                        Err(e) => {
                            push_log(&self.state.shared_logs, LogLevel::Error, &e);
                            self.state.push_toast(now, ToastKind::Error, e);
                        }
                    }
                }
                BgMsg::OperationDone {
                    kind,
                    msg,
                    refresh_tweaks,
                    refresh_system,
                    refresh_valorant,
                    refresh_emulators,
                    refresh_safety,
                    explorer_prompt,
                } => {
                    self.state.busy_task = None;
                    if explorer_prompt {
                        self.state.explorer_restart_needed = true;
                    }
                    let lvl = match kind {
                        ToastKind::Error => LogLevel::Error,
                        ToastKind::Warning => LogLevel::Warn,
                        ToastKind::Success => LogLevel::Ok,
                        ToastKind::Info => LogLevel::Info,
                    };
                    push_log(&self.state.shared_logs, lvl, &msg);
                    self.state.push_toast(now, kind, msg);
                    if refresh_tweaks {
                        self.handle_action(AppAction::AuditTweaks, ctx);
                    }
                    if refresh_system {
                        self.handle_action(AppAction::RescanSystem, ctx);
                    }
                    if refresh_valorant {
                        self.handle_action(AppAction::RefreshValorant, ctx);
                    }
                    if refresh_emulators {
                        self.handle_action(AppAction::RefreshEmulators, ctx);
                    }
                    if refresh_safety {
                        self.handle_action(AppAction::RefreshSafety, ctx);
                    }
                }
            }
        }

        if trigger_auto_reduct {
            self.handle_action(AppAction::RunMemReductClean, ctx);
        }
    }

    fn selected_adapter_name(&self) -> Option<String> {
        let ads = &self.state.system_report.network_adapters;
        ads.get(self.state.selected_adapter_idx)
            .or_else(|| ads.iter().find(|a| a.is_physical_primary))
            .or_else(|| ads.first())
            .map(|a| a.friendly_name.clone())
    }

    fn handle_action(&mut self, action: AppAction, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        match action {
            AppAction::SwitchTab(tab) => {
                self.state.active_tab = tab;
            }
            AppAction::RequestConfirm(req) => {
                self.state.pending_confirm = Some(req);
            }
            AppAction::ClearConsoleLogs => {
                if let Ok(mut g) = self.state.shared_logs.lock() {
                    g.clear();
                }
                self.state.logs_snapshot.clear();
                self.state.last_log_count = 0;
            }
            AppAction::CopyText { text, label } => {
                ctx.copy_text(text);
                self.state.push_toast(now, ToastKind::Success, label);
            }
            AppAction::OpenFolder(path) => {
                let _ = std::process::Command::new("explorer.exe").arg(&path).spawn();
            }
            AppAction::OpenUrl(url) => {
                let _ = std::process::Command::new("cmd.exe")
                    .args(["/C", "start", "", &url])
                    .spawn();
            }
            AppAction::CheckForUpdates => {
                self.state.update_status = UpdateStatus::Checking;
                self.state.busy_task = Some("Checking GitHub Releases for updates...".into());
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let res = updater::check_for_updates();
                    let _ = tx.send(BgMsg::UpdateCheckFinished(res));
                    c.request_repaint();
                });
            }
            AppAction::InstallUpdateNow(url) => {
                self.state.show_update_modal = false;
                self.state.busy_task = Some("Downloading & launching update installer...".into());
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let res = updater::download_and_launch_update(&url);
                    let _ = tx.send(BgMsg::InstallFinished(res));
                    c.request_repaint();
                });
            }
            AppAction::InstallAppToSystem => {
                self.state.busy_task =
                    Some("Installing Optimizer by Raghav to Program Files & creating shortcuts...".into());
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let res = installer::install_current_exe_to_system().map(|p| {
                        format!(
                            "Installed to {} (Desktop & Start Menu shortcuts created)",
                            p.display()
                        )
                    });
                    let _ = tx.send(BgMsg::InstallFinished(res));
                    c.request_repaint();
                });
            }
            AppAction::UninstallAppFromSystem => {
                self.state.busy_task = Some("Uninstalling from Program Files & Shortcuts...".into());
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let res = installer::uninstall_app_from_system();
                    let _ = tx.send(BgMsg::UninstallFinished(res));
                    c.request_repaint();
                });
            }
            AppAction::SaveSettings => match save_settings(&self.state.settings) {
                Ok(()) => self.state.push_toast(now, ToastKind::Success, "Settings saved"),
                Err(e) => self.state.push_toast(now, ToastKind::Error, e),
            },
            AppAction::SetUiScale(scale) => {
                let clamped = scale.clamp(0.85, 1.35);
                self.state.settings.ui_scale = clamped;
                theme::apply_scale_metrics(ctx, clamped);
                let _ = save_settings(&self.state.settings);
            }
            AppAction::ExportSpecsMarkdown => {
                let md = self.state.system_report.to_markdown_summary();
                let dir = std::env::var_os("USERPROFILE")
                    .map(|p| PathBuf::from(p).join("Desktop"))
                    .unwrap_or_else(crate::engine::app_data_dir);
                let stamp = Local::now().format("%Y%m%d_%H%M%S");
                let out_path = dir.join(format!("OptimizerByRaghav_SystemReport_{stamp}.md"));
                match std::fs::write(&out_path, md) {
                    Ok(()) => {
                        let msg = format!("Exported system report -> {}", out_path.display());
                        push_log(&self.state.shared_logs, LogLevel::Ok, &msg);
                        self.state.push_toast(now, ToastKind::Success, msg);
                    }
                    Err(e) => self
                        .state
                        .push_toast(now, ToastKind::Error, format!("Export failed: {e}")),
                }
            }
            AppAction::RescanSystem => {
                self.state.scanning_system = true;
                self.state.busy_task = Some("Scanning whole-system hardware...".into());
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let report = SystemReport::collect();
                    let _ = tx.send(BgMsg::SystemReportReady(report));
                    c.request_repaint();
                });
            }
            AppAction::AuditTweaks => {
                let is_win11 = self.state.system_report.platform.is_win11;
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let mut list = tweaks::catalog_for_os(is_win11);
                    tweaks::audit_tweaks(&mut list);
                    let _ = tx.send(BgMsg::TweaksAudited(list));
                    c.request_repaint();
                });
            }
            AppAction::ApplyPreset(preset) => {
                let label = format!("{preset:?}");
                self.state.busy_task = Some(format!("Applying {label} preset..."));
                let auto_rp = self.state.settings.auto_restore_point_before_preset;
                let ids: Vec<String> = tweaks::preset_ids(preset)
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect();
                let is_win11 = self.state.system_report.platform.is_win11;
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let valid: Vec<String> = tweaks::catalog_for_os(is_win11)
                        .into_iter()
                        .filter(|t| ids.contains(&t.id))
                        .map(|t| t.id)
                        .collect();
                    let mut rp_note = String::new();
                    if auto_rp {
                        if let Err(e) = safety::create_restore_point(&format!("Before {label}")) {
                            rp_note = format!(" (Restore Point skipped: {e})");
                        }
                    }
                    let mut j = safety::begin_transaction(format!("Preset: {label}"));
                    let mut ok_cnt = 0usize;
                    let mut err_cnt = 0usize;
                    let mut explorer_need = false;
                    for id in &valid {
                        if tweaks::apply_tweak(id, &mut j).is_ok() {
                            ok_cnt += 1;
                            if id.contains("taskbar")
                                || id.contains("context_menu")
                                || id.contains("explorer")
                            {
                                explorer_need = true;
                            }
                        } else {
                            err_cnt += 1;
                        }
                    }
                    let _ = safety::commit_transaction(&j);
                    let _ = tx.send(BgMsg::OperationDone {
                        kind: if err_cnt == 0 {
                            ToastKind::Success
                        } else {
                            ToastKind::Warning
                        },
                        msg: format!(
                            "Preset {label}: applied {ok_cnt} tweak(s), {err_cnt} skipped/failed{rp_note}"
                        ),
                        refresh_tweaks: true,
                        refresh_system: false,
                        refresh_valorant: false,
                        refresh_emulators: false,
                        refresh_safety: true,
                        explorer_prompt: explorer_need,
                    });
                    c.request_repaint();
                });
            }
            AppAction::ApplySelectedTweaks => {
                let ids: Vec<String> = self
                    .state
                    .tweaks
                    .iter()
                    .filter(|t| t.is_selected)
                    .map(|t| t.id.clone())
                    .collect();
                if ids.is_empty() {
                    return;
                }
                self.state.busy_task = Some(format!("Applying {} tweak(s)...", ids.len()));
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let mut j = safety::begin_transaction(format!("Batch {} Tweaks", ids.len()));
                    let mut ok_cnt = 0usize;
                    let mut errs = Vec::new();
                    let mut explorer_need = false;
                    for id in &ids {
                        match tweaks::apply_tweak(id, &mut j) {
                            Ok(()) => {
                                ok_cnt += 1;
                                if id.contains("taskbar")
                                    || id.contains("context_menu")
                                    || id.contains("explorer")
                                {
                                    explorer_need = true;
                                }
                            }
                            Err(e) => errs.push(format!("{id}: {e}")),
                        }
                    }
                    let _ = safety::commit_transaction(&j);
                    let kind = if errs.is_empty() {
                        ToastKind::Success
                    } else {
                        ToastKind::Warning
                    };
                    let msg = if errs.is_empty() {
                        format!(
                            "Applied {ok_cnt} tweak(s) cleanly (Journal: {})",
                            j.journal_id
                        )
                    } else {
                        format!(
                            "Applied {ok_cnt}, {} error(s): {}",
                            errs.len(),
                            errs.join("; ")
                        )
                    };
                    let _ = tx.send(BgMsg::OperationDone {
                        kind,
                        msg,
                        refresh_tweaks: true,
                        refresh_system: false,
                        refresh_valorant: false,
                        refresh_emulators: false,
                        refresh_safety: true,
                        explorer_prompt: explorer_need,
                    });
                    c.request_repaint();
                });
            }
            AppAction::ApplySingleTweak(id) => {
                self.state.busy_task = Some(format!("Applying {id}..."));
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let mut j = safety::begin_transaction(format!("Apply {id}"));
                    let res = tweaks::apply_tweak(&id, &mut j);
                    let _ = safety::commit_transaction(&j);
                    let explorer_need = id.contains("taskbar")
                        || id.contains("context_menu")
                        || id.contains("explorer");
                    let _ = tx.send(match res {
                        Ok(()) => BgMsg::OperationDone {
                            kind: ToastKind::Success,
                            msg: format!("Applied '{id}' cleanly"),
                            refresh_tweaks: true,
                            refresh_system: false,
                            refresh_valorant: false,
                            refresh_emulators: false,
                            refresh_safety: true,
                            explorer_prompt: explorer_need,
                        },
                        Err(e) => BgMsg::OperationDone {
                            kind: ToastKind::Error,
                            msg: format!("Apply '{id}' failed: {e}"),
                            refresh_tweaks: true,
                            refresh_system: false,
                            refresh_valorant: false,
                            refresh_emulators: false,
                            refresh_safety: false,
                            explorer_prompt: false,
                        },
                    });
                    c.request_repaint();
                });
            }
            AppAction::RevertSingleTweak(id) => {
                self.state.busy_task = Some(format!("Reverting {id}..."));
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let mut j = safety::begin_transaction(format!("Revert {id}"));
                    let res = tweaks::revert_tweak(&id, &mut j);
                    let _ = safety::commit_transaction(&j);
                    let explorer_need = id.contains("taskbar")
                        || id.contains("context_menu")
                        || id.contains("explorer");
                    let _ = tx.send(match res {
                        Ok(()) => BgMsg::OperationDone {
                            kind: ToastKind::Info,
                            msg: format!("Reverted '{id}' to stock"),
                            refresh_tweaks: true,
                            refresh_system: false,
                            refresh_valorant: false,
                            refresh_emulators: false,
                            refresh_safety: true,
                            explorer_prompt: explorer_need,
                        },
                        Err(e) => BgMsg::OperationDone {
                            kind: ToastKind::Error,
                            msg: format!("Revert '{id}' failed: {e}"),
                            refresh_tweaks: true,
                            refresh_system: false,
                            refresh_valorant: false,
                            refresh_emulators: false,
                            refresh_safety: false,
                            explorer_prompt: false,
                        },
                    });
                    c.request_repaint();
                });
            }
            AppAction::RestartExplorer => match tweaks::restart_explorer() {
                Ok(()) => {
                    self.state.explorer_restart_needed = false;
                    self.state
                        .push_toast(now, ToastKind::Success, "Windows Explorer restarted");
                }
                Err(e) => self.state.push_toast(now, ToastKind::Error, e),
            },
            AppAction::RunMemReductClean => {
                self.state.busy_task =
                    Some("Running Mem Reduct multi-region kernel memory clean...".into());
                let cfg = self.state.settings.mem_reduct.clone();
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let res = maintenance::clean_memory_reduct(&cfg);
                    let _ = tx.send(BgMsg::StandbyPurged(res));
                    c.request_repaint();
                });
            }
            AppAction::ClearEventLogs => {
                self.state.busy_task = Some("Clearing Windows Event Viewer logs...".into());
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let res = maintenance::clear_all_windows_event_logs();
                    let _ = tx.send(match res {
                        Ok(n) => BgMsg::OperationDone {
                            kind: ToastKind::Success,
                            msg: format!("Cleared {n} Windows Event Viewer (.evtx) channels"),
                            refresh_tweaks: false,
                            refresh_system: false,
                            refresh_valorant: false,
                            refresh_emulators: false,
                            refresh_safety: false,
                            explorer_prompt: false,
                        },
                        Err(e) => BgMsg::OperationDone {
                            kind: ToastKind::Error,
                            msg: e,
                            refresh_tweaks: false,
                            refresh_system: false,
                            refresh_valorant: false,
                            refresh_emulators: false,
                            refresh_safety: false,
                            explorer_prompt: false,
                        },
                    });
                    c.request_repaint();
                });
            }
            AppAction::EmptyRecycleBin => match maintenance::empty_recycle_bin_silent() {
                Ok(()) => {
                    self.state
                        .push_toast(now, ToastKind::Success, "Recycle Bin emptied");
                    self.handle_action(AppAction::ScanCleaner, ctx);
                }
                Err(e) => self.state.push_toast(now, ToastKind::Error, e),
            },
            AppAction::ToggleHibernation(enable) => {
                self.state.busy_task = Some("Updating hibernation state...".into());
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let res = maintenance::toggle_hibernation(enable);
                    let _ = tx.send(BgMsg::HibernationUpdated(res));
                    c.request_repaint();
                });
            }
            AppAction::ScanCleaner => {
                self.state.busy_task =
                    Some("Scanning 14 PC Cleaner categories (reparse-safe)...".into());
                let mut cats = self.state.cleaner_cats.clone();
                let age = self.state.settings.temp_cleaner_min_age_hours;
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    cleaner::scan_categories(&mut cats, age);
                    let _ = tx.send(BgMsg::CleanerScanned(cats));
                    c.request_repaint();
                });
            }
            AppAction::PurgeCleaner => {
                self.state.busy_task = Some("Cleaning selected PC Cleaner categories...".into());
                let mut cats = self.state.cleaner_cats.clone();
                let age = self.state.settings.temp_cleaner_min_age_hours;
                let tx = self.tx.clone();
                let logs = Arc::clone(&self.state.shared_logs);
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let (freed, deleted, failed, lines) =
                        cleaner::purge_selected_categories(&mut cats, age);
                    for l in lines {
                        push_log(&logs, LogLevel::Info, l);
                    }
                    let _ = tx.send(BgMsg::CleanerPurged {
                        cats,
                        freed,
                        deleted,
                        failed,
                    });
                    c.request_repaint();
                });
            }
            AppAction::CancelStreamTask => {
                self.state.cancel_flag.store(true, Ordering::SeqCst);
                self.state
                    .push_toast(now, ToastKind::Warning, "Canceling active process...");
            }
            AppAction::RunSfcScan => {
                self.start_stream_task("SFC /scannow", false, ctx, |tx_line, cancel| {
                    let code = maintenance::run_sfc_scan(move |l| tx_line(l), &cancel)?;
                    if code == 0 {
                        Ok("SFC /scannow completed cleanly (exit 0)".to_string())
                    } else {
                        Err(format!("SFC exited with code {code}"))
                    }
                });
            }
            AppAction::RunDismRestoreHealth => {
                self.start_stream_task("DISM /RestoreHealth", false, ctx, |tx_line, cancel| {
                    let code = maintenance::run_dism_restore_health(move |l| tx_line(l), &cancel)?;
                    if code == 0 {
                        Ok("DISM RestoreHealth succeeded (exit 0)".to_string())
                    } else {
                        Err(format!("DISM RestoreHealth exited with code {code}"))
                    }
                });
            }
            AppAction::RunDismCleanup { reset_base } => {
                let lbl = if reset_base {
                    "DISM /StartComponentCleanup /ResetBase"
                } else {
                    "DISM /StartComponentCleanup"
                };
                self.start_stream_task(lbl, false, ctx, move |tx_line, cancel| {
                    let code = maintenance::run_dism_component_cleanup(
                        reset_base,
                        move |l| tx_line(l),
                        &cancel,
                    )?;
                    if code == 0 {
                        Ok("DISM Component Cleanup succeeded (exit 0)".to_string())
                    } else {
                        Err(format!("DISM cleanup exited with code {code}"))
                    }
                });
            }
            AppAction::RefreshRuntimes => {
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let list = runtimes::get_runtime_catalog();
                    let _ = tx.send(BgMsg::RuntimesReady(list));
                    c.request_repaint();
                });
            }
            AppAction::InstallSelectedRuntimes => {
                let selected: Vec<(String, String)> = self
                    .state
                    .runtimes
                    .iter()
                    .filter(|r| r.is_selected)
                    .map(|r| (r.id.clone(), r.name.clone()))
                    .collect();
                if selected.is_empty() {
                    return;
                }
                self.start_stream_task(
                    "WinGet Install Selected",
                    true,
                    ctx,
                    move |tx_line, cancel| {
                        let mut ok_cnt = 0usize;
                        let mut errs = Vec::new();
                        for (id, name) in selected {
                            if cancel.load(Ordering::Relaxed) {
                                return Err("Canceled by user".into());
                            }
                            tx_line(format!("==> Installing {name} ({id}) via WinGet..."));
                            let cb = tx_line.clone();
                            match runtimes::install_package_via_winget(&id, move |l| cb(l), &cancel)
                            {
                                Ok(()) => {
                                    ok_cnt += 1;
                                    tx_line(format!("[OK] Installed {name}"));
                                }
                                Err(e) => {
                                    errs.push(format!("{id}: {e}"));
                                    tx_line(format!("[ERR] {e}"));
                                }
                            }
                        }
                        if errs.is_empty() {
                            Ok(format!("Installed {ok_cnt} package(s) cleanly"))
                        } else {
                            Err(format!(
                                "Installed {ok_cnt}, {} failed: {}",
                                errs.len(),
                                errs.join("; ")
                            ))
                        }
                    },
                );
            }
            AppAction::ScanInstalledApps => {
                self.state.scanning_apps = true;
                self.state.busy_task =
                    Some("Scanning installed Win32 & Store applications...".into());
                let inc_uwp = self.state.uninstaller_include_uwp;
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let apps = uninstaller::scan_all_installed_apps(inc_uwp);
                    let _ = tx.send(BgMsg::InstalledAppsReady(apps));
                    c.request_repaint();
                });
            }
            AppAction::StandardUninstallApp(app_id) => {
                let Some(app) = self
                    .state
                    .installed_apps
                    .iter()
                    .find(|a| a.id == app_id)
                    .cloned()
                else {
                    return;
                };
                self.state.busy_task = Some(format!(
                    "Running uninstaller for '{}' & scanning leftovers...",
                    app.display_name
                ));
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let toast = match uninstaller::run_standard_uninstall(&app) {
                        Ok(msg) => Some(msg),
                        Err(e) => Some(format!(
                            "Uninstaller error ({e}) — switched to Force Removal scan"
                        )),
                    };
                    let items = uninstaller::scan_leftovers_for_app(&app);
                    let _ = tx.send(BgMsg::LeftoversScanned { app, items, toast });
                    c.request_repaint();
                });
            }
            AppAction::ScanAppLeftovers(app_id) => {
                let Some(app) = self
                    .state
                    .installed_apps
                    .iter()
                    .find(|a| a.id == app_id)
                    .cloned()
                else {
                    return;
                };
                self.state.busy_task = Some(format!(
                    "Scanning files, folders & registry traces for '{}'...",
                    app.display_name
                ));
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let items = uninstaller::scan_leftovers_for_app(&app);
                    let _ = tx.send(BgMsg::LeftoversScanned {
                        app,
                        items,
                        toast: None,
                    });
                    c.request_repaint();
                });
            }
            AppAction::DeleteSelectedLeftovers => {
                let items = self.state.leftover_items.clone();
                let inc_uwp = self.state.uninstaller_include_uwp;
                self.state.busy_task = Some("Force-removing selected program leftovers...".into());
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let (deleted, failed, freed) = uninstaller::remove_selected_leftovers(&items);
                    let apps = uninstaller::scan_all_installed_apps(inc_uwp);
                    let _ = tx.send(BgMsg::LeftoversDeleted {
                        deleted,
                        failed,
                        freed,
                        apps,
                    });
                    c.request_repaint();
                });
            }
            AppAction::CloseLeftoversModal => {
                self.state.active_leftover_app = None;
                self.state.leftover_items.clear();
            }
            AppAction::OpenRegeditKey { hive, subkey } => {
                match uninstaller::open_in_regedit(hive, &subkey) {
                    Ok(()) => self.state.push_toast(
                        now,
                        ToastKind::Info,
                        format!("Opened Regedit at {}\\{subkey}", hive.cli_prefix()),
                    ),
                    Err(e) => self.state.push_toast(now, ToastKind::Error, e),
                }
            }
            AppAction::ScanBloat => {
                self.state.busy_task = Some("Querying Get-AppxPackage -AllUsers...".into());
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let res = debloat::scan_installed_bloat();
                    let _ = tx.send(BgMsg::BloatReady(res));
                    c.request_repaint();
                });
            }
            AppAction::RemoveSelectedBloat => {
                let items = self.state.bloat_items.clone();
                let count = items.iter().filter(|i| i.is_selected).count();
                self.state.busy_task = Some(format!("Uninstalling {count} selected UWP app(s)..."));
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let results = debloat::remove_selected_bloat(&items);
                    let ok = results.iter().filter(|(_, r)| r.is_ok()).count();
                    let fail = results.len() - ok;
                    let remaining = debloat::scan_installed_bloat();
                    let _ = tx.send(BgMsg::BloatRemoved {
                        remaining,
                        summary: format!("Debloat complete: {ok} removed, {fail} failed/skipped"),
                    });
                    c.request_repaint();
                });
            }
            AppAction::BenchmarkDns => {
                self.state.busy_task =
                    Some("Running concurrent Win32 ICMP DNS latency benchmark...".into());
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let res = network::benchmark_all_providers();
                    let _ = tx.send(BgMsg::DnsBenchmarked(res));
                    c.request_repaint();
                });
            }
            AppAction::ApplyDnsToPrimary {
                primary,
                secondary,
                name,
            } => {
                let Some(adapter) = self.selected_adapter_name() else {
                    self.state
                        .push_toast(now, ToastKind::Error, "No network adapter selected");
                    return;
                };
                self.state.busy_task = Some(format!("Setting DNS ({name}) on {adapter}..."));
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let mut j = safety::begin_transaction(format!("DNS -> {name} on {adapter}"));
                    let res = network::apply_dns_to_adapter(&mut j, &adapter, &primary, &secondary);
                    let _ = safety::commit_transaction(&j);
                    let _ = tx.send(match res {
                        Ok(()) => BgMsg::OperationDone {
                            kind: ToastKind::Success,
                            msg: format!("DNS on '{adapter}' set to {primary} / {secondary}"),
                            refresh_tweaks: false,
                            refresh_system: true,
                            refresh_valorant: false,
                            refresh_emulators: false,
                            refresh_safety: true,
                            explorer_prompt: false,
                        },
                        Err(e) => BgMsg::OperationDone {
                            kind: ToastKind::Error,
                            msg: e,
                            refresh_tweaks: false,
                            refresh_system: false,
                            refresh_valorant: false,
                            refresh_emulators: false,
                            refresh_safety: false,
                            explorer_prompt: false,
                        },
                    });
                    c.request_repaint();
                });
            }
            AppAction::ResetPrimaryDnsDhcp => {
                let Some(adapter) = self.selected_adapter_name() else {
                    return;
                };
                self.state.busy_task = Some(format!("Resetting DNS to DHCP on {adapter}..."));
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let res = network::reset_dns_dhcp(&adapter);
                    let _ = tx.send(BgMsg::OperationDone {
                        kind: if res.is_ok() {
                            ToastKind::Success
                        } else {
                            ToastKind::Error
                        },
                        msg: res
                            .map(|_| format!("Reset DNS on '{adapter}' to DHCP"))
                            .unwrap_or_else(|e| e),
                        refresh_tweaks: false,
                        refresh_system: true,
                        refresh_valorant: false,
                        refresh_emulators: false,
                        refresh_safety: false,
                        explorer_prompt: false,
                    });
                    c.request_repaint();
                });
            }
            AppAction::FlushDnsCache => match network::flush_dns_cache() {
                Ok(()) => self
                    .state
                    .push_toast(now, ToastKind::Success, "Windows DNS Resolver Cache flushed"),
                Err(e) => self.state.push_toast(now, ToastKind::Error, e),
            },
            AppAction::ResetWinsockStack => {
                self.state.busy_task = Some("Resetting Winsock & TCP/IP stack...".into());
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let res = network::reset_winsock_stack();
                    let _ = tx.send(BgMsg::OperationDone {
                        kind: if res.is_ok() {
                            ToastKind::Success
                        } else {
                            ToastKind::Error
                        },
                        msg: res.unwrap_or_else(|e| e),
                        refresh_tweaks: false,
                        refresh_system: false,
                        refresh_valorant: false,
                        refresh_emulators: false,
                        refresh_safety: false,
                        explorer_prompt: false,
                    });
                    c.request_repaint();
                });
            }
            AppAction::ToggleGameMode(on) => {
                if on {
                    self.state.game_mode_controller.start();
                    self.state
                        .push_toast(now, ToastKind::Success, "Active Game Mode daemon started");
                } else {
                    self.state.game_mode_controller.stop();
                    self.state.push_toast(
                        now,
                        ToastKind::Info,
                        "Game Mode stopped & session overrides reverted",
                    );
                }
                self.state.gamemode_status = self.state.game_mode_controller.status();
            }
            AppAction::SaveGameModeConfig => {
                self.state
                    .game_mode_controller
                    .update_config(self.state.gamemode_cfg.clone());
            }
            AppAction::AddGameModeTarget(name) => {
                match self.state.game_mode_controller.add_custom_target(&name) {
                    Ok(clean) => {
                        self.state.gamemode_cfg = self.state.game_mode_controller.get_config();
                        self.state.push_toast(
                            now,
                            ToastKind::Success,
                            format!("Added custom game target: {clean}"),
                        );
                    }
                    Err(e) => self.state.push_toast(now, ToastKind::Error, e),
                }
            }
            AppAction::RemoveGameModeTarget(name) => {
                self.state.game_mode_controller.remove_custom_target(&name);
                self.state.gamemode_cfg = self.state.game_mode_controller.get_config();
            }
            AppAction::RefreshValorant => {
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let _ = tx.send(BgMsg::ValorantReady(valorant::detect_topology()));
                    c.request_repaint();
                });
            }
            AppAction::SetValorantFsoDpi(enable) => {
                let exe = self
                    .state
                    .valorant_topo
                    .shipping_exe_path
                    .clone()
                    .unwrap_or_default();
                let mut j = safety::begin_transaction("VALORANT FSO/DPI Layer");
                let res = valorant::apply_fso_dpi(&exe, enable, &mut j);
                let _ = safety::commit_transaction(&j);
                match res {
                    Ok(()) => {
                        self.state
                            .push_toast(now, ToastKind::Success, "Updated VALORANT FSO/DPI flags");
                        self.handle_action(AppAction::RefreshValorant, ctx);
                        self.handle_action(AppAction::RefreshSafety, ctx);
                    }
                    Err(e) => self.state.push_toast(now, ToastKind::Error, e),
                }
            }
            AppAction::SetValorantQos(enable) => {
                let mut j = safety::begin_transaction("VALORANT QoS DSCP 46");
                let res = valorant::apply_qos_dscp(enable, &mut j);
                let _ = safety::commit_transaction(&j);
                match res {
                    Ok(()) => {
                        self.state.push_toast(
                            now,
                            ToastKind::Success,
                            "Updated VALORANT QoS DSCP 46 policy",
                        );
                        self.handle_action(AppAction::RefreshValorant, ctx);
                        self.handle_action(AppAction::RefreshSafety, ctx);
                    }
                    Err(e) => self.state.push_toast(now, ToastKind::Error, e),
                }
            }
            AppAction::ApplyValorantPreset(kind) => {
                let topo = self.state.valorant_topo.clone();
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let mut j =
                        safety::begin_transaction(format!("VALORANT INI Preset ({kind:?})"));
                    let res = valorant::apply_preset(&topo, kind, &mut j);
                    let _ = safety::commit_transaction(&j);
                    let _ = tx.send(match res {
                        Ok(n) => BgMsg::OperationDone {
                            kind: ToastKind::Success,
                            msg: format!(
                                "Patched {n} VALORANT GameUserSettings.ini (UTF-8 no-BOM)"
                            ),
                            refresh_tweaks: false,
                            refresh_system: false,
                            refresh_valorant: true,
                            refresh_emulators: false,
                            refresh_safety: true,
                            explorer_prompt: false,
                        },
                        Err(e) => BgMsg::OperationDone {
                            kind: ToastKind::Error,
                            msg: e,
                            refresh_tweaks: false,
                            refresh_system: false,
                            refresh_valorant: true,
                            refresh_emulators: false,
                            refresh_safety: false,
                            explorer_prompt: false,
                        },
                    });
                    c.request_repaint();
                });
            }
            AppAction::ApplyValorantBundle => {
                let topo = self.state.valorant_topo.clone();
                self.state.busy_task =
                    Some("Applying Vanguard-safe VALORANT optimization bundle...".into());
                let tx = self.tx.clone();
                let logs = Arc::clone(&self.state.shared_logs);
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let mut j = safety::begin_transaction("VALORANT Competitive Bundle");
                    let notes = valorant::apply_competitive_bundle(&topo, &mut j);
                    let _ = safety::commit_transaction(&j);
                    for n in &notes {
                        push_log(&logs, LogLevel::Info, n);
                    }
                    let _ = tx.send(BgMsg::OperationDone {
                        kind: ToastKind::Success,
                        msg: format!(
                            "VALORANT Vanguard-safe bundle applied ({} steps logged)",
                            j.steps.len()
                        ),
                        refresh_tweaks: true,
                        refresh_system: false,
                        refresh_valorant: true,
                        refresh_emulators: false,
                        refresh_safety: true,
                        explorer_prompt: false,
                    });
                    c.request_repaint();
                });
            }
            AppAction::PurgeValorantCaches => match valorant::purge_caches_safe() {
                Ok((freed, del, _)) => {
                    self.state.push_toast(
                        now,
                        ToastKind::Success,
                        format!(
                            "Purged {del} VALORANT cache/log file(s), freed {}",
                            format_bytes(freed)
                        ),
                    );
                    self.handle_action(AppAction::RefreshValorant, ctx);
                }
                Err(e) => self.state.push_toast(now, ToastKind::Error, e),
            },
            AppAction::PingRiotRegions => {
                self.state.busy_task =
                    Some("Pinging Riot global shard regions concurrently...".into());
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let res = valorant::ping_riot_regions();
                    let _ = tx.send(BgMsg::RiotPingReady(res));
                    c.request_repaint();
                });
            }
            AppAction::RefreshEmulators => {
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let _ = tx.send(BgMsg::EmulatorReady(emulator::detect_topology()));
                    c.request_repaint();
                });
            }
            AppAction::PatchEmulatorConfigs {
                fps_240,
                rog_profile,
            } => {
                let topo = self.state.emulator_topo.clone();
                self.state.busy_task =
                    Some("Extreme patching BlueStacks / MSI emulator instances...".into());
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let mut j = safety::begin_transaction("Extreme Emulator 240FPS+ROG+ASTC Patch");
                    let mut patched = 0usize;
                    let mut err_msg = None;
                    for path_opt in [&topo.bluestacks_conf, &topo.msi_conf] {
                        if let Some(p) = path_opt {
                            match emulator::patch_bluestacks_conf(
                                std::path::Path::new(p),
                                fps_240,
                                rog_profile,
                                &mut j,
                            ) {
                                Ok(()) => patched += 1,
                                Err(e) => err_msg = Some(e),
                            }
                        }
                    }
                    let _ = safety::commit_transaction(&j);
                    let msg = if let Some(e) = err_msg {
                        e
                    } else if patched == 0 {
                        "No BlueStacks / MSI bluestacks.conf files detected on system.".to_string()
                    } else {
                        format!(
                            "Extreme patched {patched} emulator config(s) [240FPS + ROG + ASTC + 4GB+ + AdBlock] across: {}",
                            topo.instances_found.join(", ")
                        )
                    };
                    let _ = tx.send(BgMsg::OperationDone {
                        kind: if patched > 0 {
                            ToastKind::Success
                        } else {
                            ToastKind::Warning
                        },
                        msg,
                        refresh_tweaks: false,
                        refresh_system: false,
                        refresh_valorant: false,
                        refresh_emulators: true,
                        refresh_safety: true,
                        explorer_prompt: false,
                    });
                    c.request_repaint();
                });
            }
            AppAction::PatchLdPlayerConfigs => {
                let cfgs = self.state.emulator_topo.ldplayer_configs.clone();
                let mut j = safety::begin_transaction("LDPlayer 9 Extreme 240FPS+ROG Patch");
                let res = emulator::patch_ldplayer_configs(&cfgs, &mut j);
                let _ = safety::commit_transaction(&j);
                match res {
                    Ok(n) => {
                        self.state.push_toast(
                            now,
                            ToastKind::Success,
                            format!("Patched {n} LDPlayer 9 VM config(s) [240FPS + ROG + ASTC]"),
                        );
                        self.handle_action(AppAction::RefreshEmulators, ctx);
                        self.handle_action(AppAction::RefreshSafety, ctx);
                    }
                    Err(e) => self.state.push_toast(now, ToastKind::Error, e),
                }
            }
            AppAction::SetEmulatorIfeoPriority(enable) => {
                let mut j = safety::begin_transaction("Emulator Kernel IFEO High Priority Lock");
                let res = emulator::apply_emulator_ifeo_priority_lock(&mut j, enable);
                let _ = safety::commit_transaction(&j);
                match res {
                    Ok(n) => {
                        self.state.push_toast(
                            now,
                            ToastKind::Success,
                            format!(
                                "Kernel IFEO High CPU & Page Priority {} for {n} emulator render engine(s)",
                                if enable { "locked" } else { "unlocked" }
                            ),
                        );
                        self.handle_action(AppAction::RefreshEmulators, ctx);
                        self.handle_action(AppAction::RefreshSafety, ctx);
                    }
                    Err(e) => self.state.push_toast(now, ToastKind::Error, e),
                }
            }
            AppAction::LockEmulatorGpu => {
                let mut j = safety::begin_transaction("Lock High-Perf GPU for Emulators");
                let res = emulator::apply_gpu_preference_lock(&mut j);
                let _ = safety::commit_transaction(&j);
                match res {
                    Ok(n) => {
                        self.state.push_toast(
                            now,
                            ToastKind::Success,
                            format!("Registered High-Perf dGPU + FSO/DPI for {n} existing emulator EXE(s)"),
                        );
                        self.handle_action(AppAction::RefreshEmulators, ctx);
                        self.handle_action(AppAction::RefreshSafety, ctx);
                    }
                    Err(e) => self.state.push_toast(now, ToastKind::Warning, e),
                }
            }
            AppAction::SetEmulatorRawMouse(on) => {
                let mut j = safety::begin_transaction("Emulator Raw 1:1 Mouse Aim");
                let res = emulator::apply_raw_mouse_aim(&mut j, on);
                let _ = safety::commit_transaction(&j);
                match res {
                    Ok(()) => {
                        self.state
                            .push_toast(now, ToastKind::Success, "Applied live mouse curve setting");
                        self.handle_action(AppAction::RefreshEmulators, ctx);
                        self.handle_action(AppAction::RefreshSafety, ctx);
                    }
                    Err(e) => self.state.push_toast(now, ToastKind::Error, e),
                }
            }
            AppAction::SetEmulatorLowLatencyNet(on) => {
                let mut j = safety::begin_transaction("Emulator Low-Latency Network");
                let res = emulator::apply_low_latency_net(&mut j, on);
                let _ = safety::commit_transaction(&j);
                match res {
                    Ok(()) => {
                        self.state.push_toast(
                            now,
                            ToastKind::Success,
                            "Updated Nagle & MMCSS for emulator low latency",
                        );
                        self.handle_action(AppAction::RefreshEmulators, ctx);
                        self.handle_action(AppAction::RefreshSafety, ctx);
                    }
                    Err(e) => self.state.push_toast(now, ToastKind::Error, e),
                }
            }
            AppAction::SetHypervisorState { turn_off } => {
                let mut j = safety::begin_transaction("Virtualization & HVCI Toggle");
                let res = emulator::set_hypervisor_state(turn_off, &mut j);
                let _ = safety::commit_transaction(&j);
                match res {
                    Ok(msg) => {
                        self.state.push_toast(now, ToastKind::Success, msg);
                        self.handle_action(AppAction::RefreshEmulators, ctx);
                        self.handle_action(AppAction::RefreshSafety, ctx);
                    }
                    Err(e) => self.state.push_toast(now, ToastKind::Error, e),
                }
            }
            AppAction::PurgeEmulatorLogs => {
                let (freed, del, _) = emulator::purge_emulator_logs_safe();
                self.state.push_toast(
                    now,
                    ToastKind::Success,
                    format!(
                        "Cleaned {del} emulator log file(s), freed {}",
                        format_bytes(freed)
                    ),
                );
                self.handle_action(AppAction::RefreshEmulators, ctx);
            }
            AppAction::RefreshSafety => {
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let _ = tx.send(BgMsg::SafetyReady {
                        journals: safety::load_all_journals(),
                        vss: safety::check_vss_health(),
                        rps: safety::list_restore_points(),
                    });
                    c.request_repaint();
                });
            }
            AppAction::RollbackJournal(jid) => {
                let Some(mut journal) = self
                    .state
                    .journals
                    .iter()
                    .find(|j| j.journal_id == jid)
                    .cloned()
                else {
                    return;
                };
                self.state.busy_task = Some(format!("Rolling back journal {jid}..."));
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let (ok, fail, errs) = safety::rollback_journal(&mut journal);
                    let _ = safety::save_journal(&journal);
                    let kind = if fail == 0 {
                        ToastKind::Success
                    } else {
                        ToastKind::Warning
                    };
                    let msg = if fail == 0 {
                        format!("Rolled back journal {jid} cleanly ({ok} steps)")
                    } else {
                        format!(
                            "Rollback partial ({ok} ok, {fail} failed): {}",
                            errs.join("; ")
                        )
                    };
                    let _ = tx.send(BgMsg::OperationDone {
                        kind,
                        msg,
                        refresh_tweaks: true,
                        refresh_system: true,
                        refresh_valorant: true,
                        refresh_emulators: true,
                        refresh_safety: true,
                        explorer_prompt: false,
                    });
                    c.request_repaint();
                });
            }
            AppAction::ExportJournalBat(jid) => {
                if let Some(j) = self.state.journals.iter().find(|x| x.journal_id == jid) {
                    match safety::export_emergency_bat(j) {
                        Ok(p) => self.state.push_toast(
                            now,
                            ToastKind::Success,
                            format!("Exported emergency undo script -> {}", p.display()),
                        ),
                        Err(e) => self.state.push_toast(now, ToastKind::Error, e),
                    }
                }
            }
            AppAction::DeleteJournal(jid) => {
                let _ = safety::delete_journal_file(&jid);
                self.handle_action(AppAction::RefreshSafety, ctx);
                self.state
                    .push_toast(now, ToastKind::Info, format!("Deleted journal {jid}"));
            }
            AppAction::CreateRestorePoint(desc) => {
                self.state.busy_task = Some("Creating Windows System Restore Point...".into());
                let tx = self.tx.clone();
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let res = safety::create_restore_point(&desc);
                    let _ = tx.send(match res {
                        Ok(()) => BgMsg::OperationDone {
                            kind: ToastKind::Success,
                            msg: format!("Created Restore Point: '{desc}'"),
                            refresh_tweaks: false,
                            refresh_system: false,
                            refresh_valorant: false,
                            refresh_emulators: false,
                            refresh_safety: true,
                            explorer_prompt: false,
                        },
                        Err(e) => BgMsg::OperationDone {
                            kind: ToastKind::Error,
                            msg: e,
                            refresh_tweaks: false,
                            refresh_system: false,
                            refresh_valorant: false,
                            refresh_emulators: false,
                            refresh_safety: true,
                            explorer_prompt: false,
                        },
                    });
                    c.request_repaint();
                });
            }
            AppAction::OpenSystemRestoreUi => {
                if let Err(e) = safety::launch_system_restore_ui() {
                    self.state.push_toast(now, ToastKind::Error, e);
                }
            }
        }
    }

    fn start_stream_task<F>(
        &mut self,
        label: &str,
        refresh_runtimes: bool,
        ctx: &egui::Context,
        worker: F,
    ) where
        F: FnOnce(Arc<dyn Fn(String) + Send + Sync>, Arc<AtomicBool>) -> Result<String, String>
            + Send
            + 'static,
    {
        self.state.cancel_flag.store(false, Ordering::SeqCst);
        self.state.stream_running = true;
        self.state.stream_label = label.to_string();
        self.state.stream_lines.clear();
        self.state.stream_lines.push(format!("[START] {label}..."));

        let cancel = Arc::clone(&self.state.cancel_flag);
        let tx = self.tx.clone();
        let tx2 = self.tx.clone();
        let c = ctx.clone();
        let c2 = ctx.clone();
        let lbl = label.to_string();

        std::thread::spawn(move || {
            let line_fn: Arc<dyn Fn(String) + Send + Sync> = Arc::new(move |line| {
                let _ = tx.send(BgMsg::StreamLine(line));
                c.request_repaint();
            });
            let result = worker(line_fn, cancel);
            let _ = tx2.send(BgMsg::StreamDone {
                label: lbl,
                result,
                refresh_runtimes,
            });
            c2.request_repaint();
        });
    }
}

impl OptimizerApp {
    fn render_resize_borders(ctx: &egui::Context) {
        let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        if is_max {
            return;
        }
        let screen = ctx.screen_rect();
        let t = 5.0;
        let edges = [
            (
                "rz_n",
                Rect::from_min_max(screen.min, Pos2::new(screen.max.x, screen.min.y + t)),
                egui::CursorIcon::ResizeNorth,
                egui::ResizeDirection::North,
            ),
            (
                "rz_s",
                Rect::from_min_max(Pos2::new(screen.min.x, screen.max.y - t), screen.max),
                egui::CursorIcon::ResizeSouth,
                egui::ResizeDirection::South,
            ),
            (
                "rz_w",
                Rect::from_min_max(screen.min, Pos2::new(screen.min.x + t, screen.max.y)),
                egui::CursorIcon::ResizeWest,
                egui::ResizeDirection::West,
            ),
            (
                "rz_e",
                Rect::from_min_max(Pos2::new(screen.max.x - t, screen.min.y), screen.max),
                egui::CursorIcon::ResizeEast,
                egui::ResizeDirection::East,
            ),
            (
                "rz_se",
                Rect::from_min_max(
                    Pos2::new(screen.max.x - 10.0, screen.max.y - 10.0),
                    screen.max,
                ),
                egui::CursorIcon::ResizeSouthEast,
                egui::ResizeDirection::SouthEast,
            ),
            (
                "rz_sw",
                Rect::from_min_max(
                    Pos2::new(screen.min.x, screen.max.y - 10.0),
                    Pos2::new(screen.min.x + 10.0, screen.max.y),
                ),
                egui::CursorIcon::ResizeSouthWest,
                egui::ResizeDirection::SouthWest,
            ),
        ];

        for (id, rect, cursor, dir) in edges {
            egui::Area::new(egui::Id::new(id))
                .fixed_pos(rect.min)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    let (_, resp) = ui.allocate_exact_size(rect.size(), egui::Sense::drag());
                    if resp.hovered() {
                        ctx.set_cursor_icon(cursor);
                    }
                    if resp.drag_started() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(dir));
                    }
                });
        }
    }
}

impl eframe::App for OptimizerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_bg_messages(ctx);

        // Zero-overhead log sync: only clone when log count actually changes
        if let Ok(g) = self.state.shared_logs.lock() {
            if g.len() != self.state.last_log_count {
                self.state.logs_snapshot = g.clone();
                self.state.last_log_count = g.len();
            }
        }

        let now = ctx.input(|i| i.time);
        self.state.toasts.retain(|t| now - t.created_at_secs < 4.5);

        let mut actions: Vec<AppAction> = Vec::new();

        // Global Hotkeys (Ctrl+K = Command Palette, Ctrl+R = Mem Reduct, F5 = Refresh, Ctrl+1..9 = Tabs, Ctrl+/- = UI Scale)
        ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::K) {
                self.state.show_command_palette = !self.state.show_command_palette;
            }
            if i.key_pressed(egui::Key::Escape) && self.state.show_command_palette {
                self.state.show_command_palette = false;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::R) {
                actions.push(AppAction::RunMemReductClean);
            }
            if i.modifiers.ctrl && (i.key_pressed(egui::Key::Equals) || i.key_pressed(egui::Key::Plus)) {
                actions.push(AppAction::SetUiScale(self.state.settings.ui_scale + 0.05));
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Minus) {
                actions.push(AppAction::SetUiScale(self.state.settings.ui_scale - 0.05));
            }
            if i.key_pressed(egui::Key::F5) {
                if self.state.active_tab == TabId::Uninstaller {
                    actions.push(AppAction::ScanInstalledApps);
                } else {
                    actions.push(AppAction::RescanSystem);
                }
            }
            if i.modifiers.ctrl {
                let key_tabs = [
                    (egui::Key::Num1, TabId::Dashboard),
                    (egui::Key::Num2, TabId::QuickBoost),
                    (egui::Key::Num3, TabId::Tweaks),
                    (egui::Key::Num4, TabId::Health),
                    (egui::Key::Num5, TabId::Uninstaller),
                    (egui::Key::Num6, TabId::Gaming),
                    (egui::Key::Num7, TabId::Network),
                    (egui::Key::Num8, TabId::Safety),
                    (egui::Key::Num9, TabId::Specs),
                    (egui::Key::Num0, TabId::Settings),
                ];
                for (k, t) in key_tabs {
                    if i.key_pressed(k) {
                        actions.push(AppAction::SwitchTab(t));
                    }
                }
            }
        });

        Self::render_resize_borders(ctx);

        // ==================== CUSTOM BORDERLESS TOP TITLE & COMMAND BAR ====================
        egui::TopBottomPanel::top("futuristic_titlebar")
            .exact_height(44.0)
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_VOID)
                    .stroke(Stroke::new(1.0_f32, theme::BORDER_HAIRLINE))
                    .inner_margin(Margin::symmetric(12, 6)),
            )
            .show(ctx, |ui| {
                let bar_rect = ui.max_rect();
                ui.horizontal_centered(|ui| {
                    // Sidebar collapse toggle
                    if icon_button(
                        ui,
                        NavIcon::SidebarToggle,
                        "Toggle Compact Rail",
                        self.state.settings.sidebar_collapsed,
                    )
                    .clicked()
                    {
                        self.state.settings.sidebar_collapsed =
                            !self.state.settings.sidebar_collapsed;
                        let _ = save_settings(&self.state.settings);
                    }

                    if let Some(tex) = &self.state.logo_tex {
                        ui.add(
                            egui::Image::new(tex)
                                .fit_to_exact_size(egui::vec2(24.0, 24.0))
                                .corner_radius(CornerRadius::same(6)),
                        );
                    }
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 5.0;
                        ui.label(
                            RichText::new("Optimizer")
                                .size(15.0)
                                .strong()
                                .color(theme::TEXT_PRIMARY),
                        );
                        ui.label(
                            RichText::new("by Raghav")
                                .size(10.5)
                                .color(theme::ACCENT_CYAN),
                        );
                    });

                    ui.add_space(10.0);

                    // Command Search Pill (Ctrl+K)
                    let cmd_btn = egui::Button::new(
                        RichText::new("  Search tweaks, apps & actions...      Ctrl+K  ")
                            .size(11.8)
                            .color(theme::TEXT_MUTED),
                    )
                    .fill(theme::BG_SURFACE)
                    .stroke(Stroke::new(1.0_f32, theme::BORDER_HAIRLINE))
                    .corner_radius(CornerRadius::same(14))
                    .min_size(Vec2::new(250.0, 27.0));
                    if ui.add(cmd_btn).clicked() {
                        self.state.show_command_palette = true;
                    }

                    if let Some(busy) = &self.state.busy_task {
                        ui.add_space(6.0);
                        ui.spinner();
                        ui.add(
                            egui::Label::new(
                                RichText::new(busy).size(11.5).color(theme::ACCENT_CYAN),
                            )
                            .truncate(),
                        );
                    }

                    // Right-side Quick Controls + Native Window Buttons
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;

                        // Close Button
                        let close_resp = ui.add(
                            egui::Button::new(
                                RichText::new("✕").size(13.0).color(theme::TEXT_SECONDARY),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE)
                            .corner_radius(CornerRadius::same(6))
                            .min_size(Vec2::new(30.0, 26.0)),
                        );
                        if close_resp.hovered() {
                            ui.painter().rect_filled(
                                close_resp.rect,
                                CornerRadius::same(6),
                                theme::ACCENT_ROSE,
                            );
                            ui.painter().text(
                                close_resp.rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "✕",
                                egui::FontId::proportional(13.0),
                                Color32::WHITE,
                            );
                        }
                        if close_resp.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }

                        // Maximize / Restore Button
                        let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(if is_max { "❐" } else { "□" })
                                        .size(12.5)
                                        .color(theme::TEXT_SECONDARY),
                                )
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE)
                                .corner_radius(CornerRadius::same(6))
                                .min_size(Vec2::new(30.0, 26.0)),
                            )
                            .clicked()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
                        }

                        // Minimize Button
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("—").size(12.0).color(theme::TEXT_SECONDARY),
                                )
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE)
                                .corner_radius(CornerRadius::same(6))
                                .min_size(Vec2::new(30.0, 26.0)),
                            )
                            .clicked()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }

                        ui.separator();

                        // Console Drawer Icon Toggle
                        if icon_button(
                            ui,
                            NavIcon::Terminal,
                            &format!("Activity Log ({})", self.state.last_log_count),
                            self.state.settings.show_console_drawer,
                        )
                        .clicked()
                        {
                            self.state.settings.show_console_drawer =
                                !self.state.settings.show_console_drawer;
                            actions.push(AppAction::SaveSettings);
                        }

                        // Update Button (Minimalist Icon / Pill)
                        match &self.state.update_status {
                            UpdateStatus::Available(info) => {
                                if emerald_button(ui, &format!("v{} Ready", info.latest_version))
                                    .clicked()
                                {
                                    self.state.show_update_modal = true;
                                }
                            }
                            UpdateStatus::Checking => {
                                ui.spinner();
                            }
                            UpdateStatus::UpToDate { version } => {
                                if icon_button(
                                    ui,
                                    NavIcon::Update,
                                    &format!("Up to date (v{version}) — Click to re-check"),
                                    false,
                                )
                                .clicked()
                                {
                                    actions.push(AppAction::CheckForUpdates);
                                }
                            }
                            UpdateStatus::Idle | UpdateStatus::Error(_) => {
                                if icon_button(
                                    ui,
                                    NavIcon::Update,
                                    "Check for Updates",
                                    false,
                                )
                                .clicked()
                                {
                                    actions.push(AppAction::CheckForUpdates);
                                }
                            }
                        }

                        // Instant RAM Purge Pill
                        let ram_pill = egui::Button::new(
                            RichText::new(format!(
                                "⚡ RAM {}%",
                                self.state.mem_stats.phys_load_pct
                            ))
                            .size(11.8)
                            .strong()
                            .monospace()
                            .color(theme::ACCENT_EMERALD),
                        )
                        .fill(theme::ACCENT_EMERALD_DIM)
                        .stroke(Stroke::new(
                            1.0_f32,
                            theme::ACCENT_EMERALD.linear_multiply(0.55),
                        ))
                        .corner_radius(CornerRadius::same(14))
                        .min_size(Vec2::new(86.0, 26.0));
                        if ui
                            .add(ram_pill)
                            .on_hover_text("Click to clean memory now (Ctrl+R)")
                            .clicked()
                        {
                            actions.push(AppAction::RunMemReductClean);
                        }

                        // Live CPU & Timer micro-readout
                        ui.label(
                            RichText::new(format!(
                                "CPU {:>2.0}%  •  {:.1}ms",
                                self.state.live_cpu_pct,
                                self.state.gamemode_status.current_timer_ms
                            ))
                            .size(11.2)
                            .monospace()
                            .color(theme::TEXT_MUTED),
                        );
                    });
                });

                // Drag empty area of titlebar to move window; double-click to maximize
                let drag_resp = ui.interact(
                    bar_rect,
                    egui::Id::new("titlebar_drag_zone"),
                    egui::Sense::click_and_drag(),
                );
                if drag_resp.double_clicked() {
                    let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
                } else if drag_resp.drag_started_by(egui::PointerButton::Primary) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
            });

        // ==================== ADAPTIVE COLLAPSIBLE NAVIGATION RAIL ====================
        let auto_compact = ctx.screen_rect().width() < 1040.0;
        let collapsed = self.state.settings.sidebar_collapsed || auto_compact;
        let rail_width = if collapsed { 62.0 } else { 184.0 };

        egui::SidePanel::left("nav_sidebar")
            .exact_width(rail_width)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_RAIL)
                    .stroke(Stroke::new(1.0_f32, theme::BORDER_HAIRLINE))
                    .inner_margin(Margin::symmetric(if collapsed { 8 } else { 10 }, 12)),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;

                for &tab in TabId::ALL {
                    if tab.divider_before() {
                        ui.add_space(4.0);
                        let y = ui.cursor().top();
                        let r = ui.available_rect_before_wrap();
                        ui.painter().line_segment(
                            [Pos2::new(r.left() + 6.0, y), Pos2::new(r.right() - 6.0, y)],
                            Stroke::new(1.0_f32, theme::BORDER_HAIRLINE),
                        );
                        ui.add_space(5.0);
                    }

                    let selected = self.state.active_tab == tab;
                    let row_h = 36.0;
                    let (rect, resp) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), row_h),
                        egui::Sense::click(),
                    );

                    if selected {
                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::same(9),
                            theme::ACCENT_CYAN_DIM,
                        );
                        ui.painter().rect_stroke(
                            rect,
                            CornerRadius::same(9),
                            Stroke::new(1.0_f32, theme::ACCENT_CYAN.linear_multiply(0.55)),
                            egui::StrokeKind::Inside,
                        );
                        let bar = Rect::from_min_max(
                            Pos2::new(rect.left() + 2.0, rect.top() + 8.0),
                            Pos2::new(rect.left() + 5.0, rect.bottom() - 8.0),
                        );
                        ui.painter()
                            .rect_filled(bar, CornerRadius::same(2), theme::ACCENT_CYAN);
                    } else if resp.hovered() {
                        ui.painter()
                            .rect_filled(rect, CornerRadius::same(9), theme::BG_HOVER);
                    }

                    let icon_col = if selected {
                        theme::ACCENT_CYAN
                    } else if resp.hovered() {
                        theme::TEXT_PRIMARY
                    } else {
                        theme::TEXT_SECONDARY
                    };

                    if collapsed {
                        paint_icon(ui.painter(), rect.center(), 17.0, tab.icon(), icon_col);
                    } else {
                        let icon_pos = Pos2::new(rect.left() + 22.0, rect.center().y);
                        paint_icon(ui.painter(), icon_pos, 16.0, tab.icon(), icon_col);
                        let txt_col = if selected {
                            theme::TEXT_PRIMARY
                        } else {
                            theme::TEXT_SECONDARY
                        };
                        ui.painter().text(
                            Pos2::new(rect.left() + 42.0, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            tab.label(),
                            egui::FontId::proportional(13.2),
                            txt_col,
                        );
                    }

                    let resp = if collapsed {
                        resp.on_hover_text(tab.label())
                    } else {
                        resp
                    };
                    if resp.clicked() {
                        actions.push(AppAction::SwitchTab(tab));
                    }
                }

                // Minimalist bottom status indicator
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(2.0);
                    let dot_col = if self.state.is_elevated {
                        theme::ACCENT_EMERALD
                    } else {
                        theme::ACCENT_AMBER
                    };
                    if collapsed {
                        let (r, resp) =
                            ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
                        ui.painter().circle_filled(r.center(), 4.0, dot_col);
                        resp.on_hover_text(if self.state.is_elevated {
                            format!("Administrator • v{APP_VERSION}")
                        } else {
                            format!("Standard User • v{APP_VERSION}")
                        });
                    } else {
                        ui.horizontal(|ui| {
                            let (r, _) =
                                ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
                            ui.painter().circle_filled(r.center(), 3.5, dot_col);
                            ui.label(
                                RichText::new(format!(
                                    "{} • v{APP_VERSION}",
                                    if self.state.is_elevated {
                                        "Admin"
                                    } else {
                                        "User"
                                    }
                                ))
                                .size(11.0)
                                .color(theme::TEXT_MUTED),
                            );
                        });
                    }
                });
            });

        // ==================== BOTTOM ACTIVITY CONSOLE DRAWER ====================
        if self.state.settings.show_console_drawer {
            egui::TopBottomPanel::bottom("console_drawer")
                .resizable(true)
                .default_height(145.0)
                .min_height(85.0)
                .max_height(300.0)
                .frame(
                    egui::Frame::new()
                        .fill(theme::BG_VOID)
                        .stroke(Stroke::new(1.0_f32, theme::BORDER_SUBTLE))
                        .inner_margin(Margin::symmetric(14, 8)),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("ACTIVITY LOG")
                                .size(10.5)
                                .strong()
                                .color(theme::TEXT_MUTED),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if subtle_button(ui, "Close").clicked() {
                                self.state.settings.show_console_drawer = false;
                                actions.push(AppAction::SaveSettings);
                            }
                            if subtle_button(ui, "Clear").clicked() {
                                actions.push(AppAction::ClearConsoleLogs);
                            }
                            if subtle_button(ui, "Copy").clicked() {
                                let joined: String = self
                                    .state
                                    .logs_snapshot
                                    .iter()
                                    .map(|e| {
                                        format!("[{}] [{:?}] {}", e.timestamp, e.level, e.message)
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                actions.push(AppAction::CopyText {
                                    text: joined,
                                    label: "Activity log copied".to_string(),
                                });
                            }
                        });
                    });
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .id_salt("global_console_scroll")
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for entry in &self.state.logs_snapshot {
                                let c = match entry.level {
                                    LogLevel::Ok => theme::ACCENT_EMERALD,
                                    LogLevel::Warn => theme::ACCENT_AMBER,
                                    LogLevel::Error => theme::ACCENT_ROSE,
                                    LogLevel::Info => theme::TEXT_SECONDARY,
                                };
                                ui.label(
                                    RichText::new(format!(
                                        "{}  {}",
                                        entry.timestamp, entry.message
                                    ))
                                    .size(11.5)
                                    .monospace()
                                    .color(c),
                                );
                            }
                        });
                });
        }

        // ==================== CENTRAL CONTENT CANVAS ====================
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_CANVAS)
                    .inner_margin(Margin::symmetric(24, 18)),
            )
            .show(ctx, |ui| {
                if self.state.active_tab == TabId::Uninstaller
                    && self.state.apps_subview == AppsSubView::Installed
                {
                    tabs::uninstaller::render(ui, &mut self.state, &mut actions);
                } else {
                    egui::ScrollArea::vertical()
                        .id_salt("main_tab_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| match self.state.active_tab {
                            TabId::Dashboard => {
                                tabs::dashboard::render(ui, &self.state, &mut actions)
                            }
                            TabId::QuickBoost => {
                                tabs::quick_boost::render(ui, &self.state, &mut actions)
                            }
                            TabId::Tweaks => {
                                tabs::tweaks::render(ui, &mut self.state, &mut actions)
                            }
                            TabId::Health => {
                                tabs::health::render(ui, &mut self.state, &mut actions)
                            }
                            TabId::Uninstaller => {
                                tabs::uninstaller::render(ui, &mut self.state, &mut actions)
                            }
                            TabId::Gaming => {
                                tabs::gaming::render(ui, &mut self.state, &mut actions)
                            }
                            TabId::Network => {
                                tabs::network::render(ui, &mut self.state, &mut actions)
                            }
                            TabId::Safety => {
                                tabs::safety::render(ui, &mut self.state, &mut actions)
                            }
                            TabId::Specs => tabs::specs::render(ui, &self.state, &mut actions),
                            TabId::Settings => {
                                tabs::settings::render(ui, &mut self.state, &mut actions)
                            }
                        });
                }
            });

        // ==================== COMMAND PALETTE OVERLAY (CTRL+K) ====================
        if self.state.show_command_palette {
            let screen = ctx.screen_rect();
            egui::Area::new(egui::Id::new("cmd_palette_scrim"))
                .fixed_pos(screen.min)
                .order(egui::Order::Middle)
                .show(ctx, |ui| {
                    let resp = ui.allocate_response(screen.size(), egui::Sense::click());
                    ui.painter().rect_filled(
                        screen,
                        CornerRadius::ZERO,
                        Color32::from_rgba_unmultiplied(4, 6, 10, 180),
                    );
                    if resp.clicked() {
                        self.state.show_command_palette = false;
                    }
                });

            let mut close_palette = false;
            egui::Window::new("Command Palette")
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 84.0))
                .order(egui::Order::Tooltip)
                .fixed_size(egui::vec2(500.0, 320.0))
                .show(ctx, |ui| {
                    search_bar(
                        ui,
                        &mut self.state.command_query,
                        "Jump to page, run optimization, or toggle tweak...",
                        468.0,
                    );
                    ui.add_space(8.0);
                    let q = self.state.command_query.trim().to_ascii_lowercase();
                    egui::ScrollArea::vertical()
                        .max_height(255.0)
                        .show(ui, |ui| {
                            let quick_cmds: &[(&str, AppAction)] = &[
                                ("Clean Memory Now (Mem Reduct)", AppAction::RunMemReductClean),
                                (
                                    "Apply Extreme Win11 & eSports Preset",
                                    AppAction::ApplyPreset(crate::models::PresetId::ExtremeWin11),
                                ),
                                (
                                    "Apply Competitive Low-Latency Preset",
                                    AppAction::ApplyPreset(
                                        crate::models::PresetId::CompetitiveLowLatency,
                                    ),
                                ),
                                ("Scan PC Junk & Caches", AppAction::ScanCleaner),
                                ("Run DNS Latency Benchmark", AppAction::BenchmarkDns),
                                ("Rescan System Hardware", AppAction::RescanSystem),
                                ("Check for Software Updates", AppAction::CheckForUpdates),
                            ];

                            for (label, act) in quick_cmds {
                                if q.is_empty() || label.to_ascii_lowercase().contains(&q) {
                                    let btn = egui::Button::new(
                                        RichText::new(format!("⚡  {label}"))
                                            .size(12.8)
                                            .color(theme::TEXT_PRIMARY),
                                    )
                                    .fill(theme::BG_ELEVATED)
                                    .min_size(Vec2::new(ui.available_width(), 30.0));
                                    if ui.add(btn).clicked() {
                                        actions.push(act.clone());
                                        close_palette = true;
                                    }
                                }
                            }

                            for &tab in TabId::ALL {
                                let lbl = format!("Go to {}", tab.label());
                                if q.is_empty() || lbl.to_ascii_lowercase().contains(&q) {
                                    let btn = egui::Button::new(
                                        RichText::new(format!("→  {lbl}"))
                                            .size(12.5)
                                            .color(theme::TEXT_SECONDARY),
                                    )
                                    .fill(Color32::TRANSPARENT)
                                    .min_size(Vec2::new(ui.available_width(), 28.0));
                                    if ui.add(btn).clicked() {
                                        actions.push(AppAction::SwitchTab(tab));
                                        close_palette = true;
                                    }
                                }
                            }

                            if !q.is_empty() {
                                for t in &self.state.tweaks {
                                    if t.title.to_ascii_lowercase().contains(&q) {
                                        let verb = if t.is_applied { "Revert" } else { "Apply" };
                                        let btn = egui::Button::new(
                                            RichText::new(format!("{verb}: {}", t.title))
                                                .size(12.5)
                                                .color(theme::ACCENT_CYAN),
                                        )
                                        .fill(Color32::TRANSPARENT)
                                        .min_size(Vec2::new(ui.available_width(), 28.0));
                                        if ui.add(btn).clicked() {
                                            if t.is_applied {
                                                actions.push(AppAction::RevertSingleTweak(
                                                    t.id.clone(),
                                                ));
                                            } else {
                                                actions.push(AppAction::ApplySingleTweak(
                                                    t.id.clone(),
                                                ));
                                            }
                                            close_palette = true;
                                        }
                                    }
                                }
                            }
                        });
                });
            if close_palette {
                self.state.show_command_palette = false;
                self.state.command_query.clear();
            }
        }

        // ==================== UPDATE AVAILABLE MODAL ====================
        if self.state.show_update_modal {
            if let UpdateStatus::Available(info) = self.state.update_status.clone() {
                let screen = ctx.screen_rect();
                egui::Area::new(egui::Id::new("update_scrim"))
                    .fixed_pos(screen.min)
                    .order(egui::Order::Middle)
                    .show(ctx, |ui| {
                        ui.allocate_response(screen.size(), egui::Sense::click());
                        ui.painter().rect_filled(
                            screen,
                            CornerRadius::ZERO,
                            Color32::from_rgba_unmultiplied(4, 6, 10, 190),
                        );
                    });

                let mut close_update = false;
                egui::Window::new(format!("Update v{}", info.latest_version))
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                    .order(egui::Order::Tooltip)
                    .fixed_size(egui::vec2(460.0, 220.0))
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            status_badge(
                                ui,
                                &format!("v{}", info.current_version),
                                theme::TEXT_SECONDARY,
                                theme::BG_ELEVATED,
                            );
                            ui.label(RichText::new("→").color(theme::TEXT_MUTED));
                            status_badge(
                                ui,
                                &format!("v{}", info.latest_version),
                                theme::ACCENT_EMERALD,
                                theme::ACCENT_EMERALD_DIM,
                            );
                        });
                        ui.add_space(8.0);
                        egui::ScrollArea::vertical()
                            .max_height(100.0)
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(&info.release_notes)
                                        .size(12.2)
                                        .color(theme::TEXT_SECONDARY),
                                );
                            });
                        ui.add_space(12.0);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if emerald_button(ui, &format!("Install v{}", info.latest_version))
                                .clicked()
                            {
                                actions.push(AppAction::InstallUpdateNow(
                                    info.setup_download_url.clone(),
                                ));
                                close_update = true;
                            }
                            if subtle_button(ui, "Release Notes").clicked() {
                                actions.push(AppAction::OpenUrl(info.release_url.clone()));
                            }
                            if subtle_button(ui, "Later").clicked() {
                                close_update = true;
                            }
                        });
                    });
                if close_update {
                    self.state.show_update_modal = false;
                }
            } else {
                self.state.show_update_modal = false;
            }
        }

        // ==================== TOAST OVERLAY (BOTTOM-RIGHT FLOATING PILLS) ====================
        if !self.state.toasts.is_empty() {
            egui::Area::new(egui::Id::new("toast_overlay"))
                .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-20.0, -20.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        let mut dismiss_idx = None;
                        for (idx, t) in self.state.toasts.iter().enumerate() {
                            let accent = match t.kind {
                                ToastKind::Success => theme::ACCENT_EMERALD,
                                ToastKind::Warning => theme::ACCENT_AMBER,
                                ToastKind::Error => theme::ACCENT_ROSE,
                                ToastKind::Info => theme::ACCENT_CYAN,
                            };
                            egui::Frame::new()
                                .fill(theme::BG_ELEVATED)
                                .stroke(Stroke::new(1.0_f32, accent.linear_multiply(0.65)))
                                .corner_radius(CornerRadius::same(10))
                                .inner_margin(Margin::symmetric(14, 9))
                                .show(ui, |ui| {
                                    ui.set_max_width(360.0);
                                    ui.horizontal(|ui| {
                                        let (r, _) = ui.allocate_exact_size(
                                            Vec2::splat(8.0),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().circle_filled(r.center(), 3.5, accent);
                                        ui.label(
                                            RichText::new(&t.text)
                                                .size(12.2)
                                                .color(theme::TEXT_PRIMARY),
                                        );
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new("×")
                                                        .size(13.0)
                                                        .color(theme::TEXT_MUTED),
                                                )
                                                .frame(false),
                                            )
                                            .clicked()
                                        {
                                            dismiss_idx = Some(idx);
                                        }
                                    });
                                });
                            ui.add_space(6.0);
                        }
                        if let Some(i) = dismiss_idx {
                            self.state.toasts.remove(i);
                        }
                    });
                });
        }

        // ==================== CONFIRMATION MODAL OVERLAY ====================
        if let Some(req) = self.state.pending_confirm.clone() {
            let screen = ctx.screen_rect();
            egui::Area::new(egui::Id::new("confirm_scrim"))
                .fixed_pos(screen.min)
                .order(egui::Order::Middle)
                .show(ctx, |ui| {
                    ui.allocate_response(screen.size(), egui::Sense::click());
                    ui.painter().rect_filled(
                        screen,
                        CornerRadius::ZERO,
                        Color32::from_rgba_unmultiplied(4, 6, 10, 190),
                    );
                });

            let mut close_modal = false;
            egui::Window::new(&req.title)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .order(egui::Order::Tooltip)
                .fixed_size(egui::vec2(440.0, 185.0))
                .show(ctx, |ui| {
                    ui.label(
                        RichText::new(&req.message)
                            .size(12.8)
                            .color(theme::TEXT_SECONDARY),
                    );
                    ui.add_space(16.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if danger_button(ui, &req.confirm_label).clicked() {
                            actions.push(*req.action);
                            close_modal = true;
                        }
                        if subtle_button(ui, "Cancel").clicked() {
                            close_modal = true;
                        }
                    });
                });
            if close_modal {
                self.state.pending_confirm = None;
            }
        }

        for act in actions {
            self.handle_action(act, ctx);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.telemetry_running.store(false, Ordering::Relaxed);
        self.state.game_mode_controller.stop();
    }
}
