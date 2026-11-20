pub mod components;
pub mod tabs;
pub mod theme;

use crate::engine::gamemode::GameModeController;
use crate::engine::maintenance::{HibernationStatus, StandbyPurgeResult};
use crate::engine::SharedLogs;
use crate::models::{
    AppSettings, BloatAppItem, CleanerCategory, DnsProviderResult, EmulatorTopology,
    GameModeConfig, GameModeStatus, InstalledAppEntry, InstalledAppKind, LeftoverItem, LogEntry,
    MemReductLiveStats, PresetId, RegHive, RestorePointInfo, RiotRegionPing, RuntimePackageItem,
    SystemReport, ToastKind, ToastMessage, TransactionJournal, TweakCategory, TweakViewItem,
    UpdateStatus, ValorantPresetKind, ValorantTopology, VssHealth,
};
use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabId {
    Dashboard,
    Specs,
    QuickBoost,
    Tweaks,
    Health,
    Uninstaller,
    Debloat,
    Network,
    Gaming,
    Safety,
    Settings,
}

impl TabId {
    pub const ALL: &'static [TabId] = &[
        TabId::Dashboard,
        TabId::Specs,
        TabId::QuickBoost,
        TabId::Tweaks,
        TabId::Health,
        TabId::Uninstaller,
        TabId::Debloat,
        TabId::Network,
        TabId::Gaming,
        TabId::Safety,
        TabId::Settings,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Specs => "System Specs",
            Self::QuickBoost => "Quick Boost",
            Self::Tweaks => "Win11 & Tweaks",
            Self::Health => "Mem Reduct & Cleaner",
            Self::Uninstaller => "Geek Uninstaller",
            Self::Debloat => "UWP Debloat",
            Self::Network => "Network",
            Self::Gaming => "Gaming & Emulators",
            Self::Safety => "Safety Center",
            Self::Settings => "Settings & Updates",
        }
    }

    pub fn tag(self) -> &'static str {
        match self {
            Self::Dashboard => "01",
            Self::Specs => "02",
            Self::QuickBoost => "03",
            Self::Tweaks => "04",
            Self::Health => "05",
            Self::Uninstaller => "06",
            Self::Debloat => "07",
            Self::Network => "08",
            Self::Gaming => "09",
            Self::Safety => "10",
            Self::Settings => "11",
        }
    }

    pub fn section_heading_before(self) -> Option<&'static str> {
        match self {
            Self::Dashboard => Some("OVERVIEW"),
            Self::QuickBoost => Some("OPTIMIZE & CLEAN"),
            Self::Network => Some("GAMING & SAFETY"),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfirmRequest {
    pub title: String,
    pub message: String,
    pub confirm_label: String,
    pub action: Box<AppAction>,
}

#[derive(Debug, Clone)]
pub enum AppAction {
    SwitchTab(TabId),
    RescanSystem,
    CopyText { text: String, label: String },
    ExportSpecsMarkdown,
    AuditTweaks,
    ApplyPreset(PresetId),
    ApplySelectedTweaks,
    ApplySingleTweak(String),
    RevertSingleTweak(String),
    RestartExplorer,
    RunMemReductClean,
    ClearEventLogs,
    EmptyRecycleBin,
    ToggleHibernation(bool),
    ScanCleaner,
    PurgeCleaner,
    RunSfcScan,
    RunDismRestoreHealth,
    RunDismCleanup { reset_base: bool },
    CancelStreamTask,
    RefreshRuntimes,
    InstallSelectedRuntimes,
    ScanInstalledApps,
    StandardUninstallApp(String),
    ScanAppLeftovers(String),
    DeleteSelectedLeftovers,
    CloseLeftoversModal,
    OpenRegeditKey { hive: RegHive, subkey: String },
    ScanBloat,
    RemoveSelectedBloat,
    BenchmarkDns,
    ApplyDnsToPrimary { primary: String, secondary: String, name: String },
    ResetPrimaryDnsDhcp,
    FlushDnsCache,
    ResetWinsockStack,
    ToggleGameMode(bool),
    SaveGameModeConfig,
    AddGameModeTarget(String),
    RemoveGameModeTarget(String),
    RefreshValorant,
    SetValorantFsoDpi(bool),
    SetValorantQos(bool),
    ApplyValorantPreset(ValorantPresetKind),
    ApplyValorantBundle,
    PurgeValorantCaches,
    PingRiotRegions,
    RefreshEmulators,
    PatchEmulatorConfigs { fps_240: bool, rog_profile: bool },
    PatchLdPlayerConfigs,
    SetEmulatorIfeoPriority(bool),
    LockEmulatorGpu,
    SetEmulatorRawMouse(bool),
    SetEmulatorLowLatencyNet(bool),
    SetHypervisorState { turn_off: bool },
    PurgeEmulatorLogs,
    RefreshSafety,
    RollbackJournal(String),
    ExportJournalBat(String),
    DeleteJournal(String),
    CreateRestorePoint(String),
    OpenSystemRestoreUi,
    CheckForUpdates,
    InstallUpdateNow(String),
    OpenUrl(String),
    InstallAppToSystem,
    UninstallAppFromSystem,
    SaveSettings,
    OpenFolder(String),
    RequestConfirm(ConfirmRequest),
    ClearConsoleLogs,
}

pub struct UiState {
    pub active_tab: TabId,
    pub is_elevated: bool,
    pub is_installed_on_pc: bool,
    pub update_status: UpdateStatus,
    pub show_update_modal: bool,
    pub settings: AppSettings,
    pub logo_tex: Option<egui::TextureHandle>,

    pub system_report: SystemReport,
    pub scanning_system: bool,

    pub cpu_history: VecDeque<f32>,
    pub ram_history: VecDeque<f32>,
    pub live_cpu_pct: f32,
    pub live_ram_used_bytes: u64,
    pub live_ram_total_bytes: u64,
    pub mem_stats: MemReductLiveStats,

    pub tweaks: Vec<TweakViewItem>,
    pub tweaks_search: String,
    pub tweaks_category: Option<TweakCategory>,
    pub explorer_restart_needed: bool,

    pub cleaner_cats: Vec<CleanerCategory>,
    pub hibernation: HibernationStatus,
    pub last_standby_purge: Option<StandbyPurgeResult>,
    pub stream_lines: Vec<String>,
    pub stream_running: bool,
    pub stream_label: String,
    pub runtimes: Vec<RuntimePackageItem>,

    pub installed_apps: Vec<InstalledAppEntry>,
    pub uninstaller_search: String,
    pub uninstaller_filter_kind: Option<InstalledAppKind>,
    pub uninstaller_include_uwp: bool,
    pub uninstaller_sort_by_size: bool,
    pub active_leftover_app: Option<InstalledAppEntry>,
    pub leftover_items: Vec<LeftoverItem>,
    pub scanning_apps: bool,

    pub bloat_items: Vec<BloatAppItem>,
    pub bloat_search: String,

    pub dns_providers: Vec<DnsProviderResult>,
    pub dns_custom_primary: String,
    pub dns_custom_secondary: String,
    pub selected_adapter_idx: usize,

    pub gamemode_cfg: GameModeConfig,
    pub gamemode_status: GameModeStatus,
    pub gamemode_custom_input: String,
    pub valorant_topo: ValorantTopology,
    pub riot_pings: Vec<RiotRegionPing>,
    pub emulator_topo: EmulatorTopology,

    pub journals: Vec<TransactionJournal>,
    pub vss_health: VssHealth,
    pub restore_points: Vec<RestorePointInfo>,
    pub restore_desc_input: String,

    pub shared_logs: SharedLogs,
    pub logs_snapshot: Vec<LogEntry>,
    pub last_log_count: usize,
    pub toasts: Vec<ToastMessage>,
    pub pending_confirm: Option<ConfirmRequest>,
    pub busy_task: Option<String>,
    pub cancel_flag: Arc<AtomicBool>,
    pub game_mode_controller: Arc<GameModeController>,
}

impl UiState {
    pub fn push_toast(&mut self, ctx_time: f64, kind: ToastKind, text: impl Into<String>) {
        self.toasts.push(ToastMessage {
            text: text.into(),
            kind,
            created_at_secs: ctx_time,
        });
        if self.toasts.len() > 5 {
            self.toasts.remove(0);
        }
    }
}
