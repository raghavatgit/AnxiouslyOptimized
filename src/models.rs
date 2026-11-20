use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Info,
    Ok,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct ToastMessage {
    pub text: String,
    pub kind: ToastKind,
    pub created_at_secs: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValorantInstallStatus {
    NotInstalled,
    InstalledNotRunning,
    Running,
}

impl Default for ValorantInstallStatus {
    fn default() -> Self {
        Self::NotInstalled
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CpuInfo {
    pub brand: String,
    pub vendor: String,
    pub architecture: String,
    pub socket: String,
    pub base_mhz: u32,
    pub max_mhz: u32,
    pub physical_cores: usize,
    pub logical_threads: usize,
    pub p_core_count: usize,
    pub e_core_count: usize,
    pub p_core_mask: u64,
    pub x3d_ccd0_mask: u64,
    pub is_hybrid: bool,
    pub is_x3d: bool,
    pub smt_enabled: bool,
    pub l1_kb: u32,
    pub l2_kb: u32,
    pub l3_kb: u32,
    pub topology_label: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub vendor: String,
    pub vendor_id: u32,
    pub device_id: u32,
    pub vram_bytes: u64,
    pub shared_sys_mem_bytes: u64,
    pub driver_version: String,
    pub driver_date: String,
    pub is_integrated: bool,
    pub pnp_device_id: Option<String>,
    pub msi_supported: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DisplayInfo {
    pub device_name: String,
    pub monitor_name: String,
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
    pub bits_per_pixel: u32,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DimmStick {
    pub slot: String,
    pub manufacturer: String,
    pub part_number: String,
    pub capacity_bytes: u64,
    pub configured_mts: u32,
    pub rated_mts: u32,
    pub memory_type: String,
    pub form_factor: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_phys_bytes: u64,
    pub avail_phys_bytes: u64,
    pub used_phys_bytes: u64,
    pub memory_load_pct: u32,
    pub commit_total_bytes: u64,
    pub commit_limit_bytes: u64,
    pub system_cache_bytes: u64,
    pub total_slots: u32,
    pub used_slots: usize,
    pub speed_mts: u32,
    pub memory_type: String,
    pub channel_mode: String,
    pub dimms: Vec<DimmStick>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhysicalDiskInfo {
    pub friendly_name: String,
    pub media_type: String,
    pub bus_type: String,
    pub health_status: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolumeInfo {
    pub mount: String,
    pub label: String,
    pub filesystem: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
    pub used_pct: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub os_name: String,
    pub edition: String,
    pub display_version: String,
    pub build_number: u32,
    pub ubr: u32,
    pub is_win11: bool,
    pub architecture: String,
    pub computer_name: String,
    pub user_name: String,
    pub uptime_secs: u64,
    pub pending_reboot: bool,
    pub pending_reboot_reasons: Vec<String>,
    pub motherboard: String,
    pub bios_vendor: String,
    pub bios_version: String,
    pub bios_date: String,
    pub chassis_type: String,
    pub is_laptop: bool,
    pub battery_present: bool,
    pub battery_pct: Option<u8>,
    pub ac_online: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecurityInfo {
    pub secure_boot: Option<bool>,
    pub tpm_present: bool,
    pub tpm_enabled: bool,
    pub tpm_spec_version: String,
    pub hvci_enabled: bool,
    pub hypervisor_launch_type: String,
    pub vbs_active: bool,
    pub game_mode_enabled: bool,
    pub game_dvr_enabled: bool,
    pub hags_enabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PowerInfo {
    pub active_scheme_guid: String,
    pub active_scheme_name: String,
    pub timer_res_current_ms: f64,
    pub timer_res_min_ms: f64,
    pub timer_res_max_ms: f64,
    pub hibernation_enabled: bool,
    pub hiberfil_bytes: u64,
    pub system_drive: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkAdapterInfo {
    pub friendly_name: String,
    pub description: String,
    pub guid: String,
    pub adapter_kind: String,
    pub is_physical_primary: bool,
    pub link_speed_mbps: u64,
    pub ipv4_addrs: Vec<String>,
    pub ipv6_addrs: Vec<String>,
    pub gateways: Vec<String>,
    pub dns_servers: Vec<String>,
    pub mac_address: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EcosystemInfo {
    pub valorant_status: ValorantInstallStatus,
    pub valorant_path: Option<String>,
    pub riot_client_path: Option<String>,
    pub steam_installed: bool,
    pub epic_installed: bool,
    pub bluestacks_version: Option<String>,
    pub msi_player_version: Option<String>,
    pub ldplayer_installed: bool,
    pub winget_available: bool,
    pub winget_version: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemReport {
    pub cpu: CpuInfo,
    pub gpus: Vec<GpuInfo>,
    pub displays: Vec<DisplayInfo>,
    pub memory: MemoryInfo,
    pub physical_disks: Vec<PhysicalDiskInfo>,
    pub volumes: Vec<VolumeInfo>,
    pub platform: PlatformInfo,
    pub security: SecurityInfo,
    pub power: PowerInfo,
    pub network_adapters: Vec<NetworkAdapterInfo>,
    pub ecosystem: EcosystemInfo,
    pub diagnostics: Vec<String>,
    pub scanned_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TweakCategory {
    WindowsUi,
    Privacy,
    Gaming,
    Speed,
    Network,
}

impl TweakCategory {
    pub fn label(self) -> &'static str {
        match self {
            Self::WindowsUi => "Windows UI",
            Self::Privacy => "Privacy",
            Self::Gaming => "Gaming & Input",
            Self::Speed => "Speed & Hardware",
            Self::Network => "Network",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TweakRisk {
    Safe,
    Moderate,
    Advanced,
}

impl TweakRisk {
    pub fn label(self) -> &'static str {
        match self {
            Self::Safe => "SAFE",
            Self::Moderate => "MODERATE",
            Self::Advanced => "ADVANCED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TweakOsTarget {
    Win11Only,
    Win10Only,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TweakViewItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: TweakCategory,
    pub risk: TweakRisk,
    pub os_target: TweakOsTarget,
    pub requires_restart: bool,
    pub requires_explorer_restart: bool,
    pub is_applied: bool,
    pub is_selected: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresetId {
    EverydayBoost,
    GamingRig,
    CompetitiveLowLatency,
    ExtremeWin11,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegHive {
    Hklm,
    Hkcu,
}

impl RegHive {
    pub fn cli_prefix(self) -> &'static str {
        match self {
            Self::Hklm => "HKLM",
            Self::Hkcu => "HKCU",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypedRegValue {
    Dword(u32),
    Qword(u64),
    String(String),
    ExpandString(String),
    MultiString(Vec<String>),
    Binary(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStep {
    RegistryValue {
        hive: RegHive,
        subkey: String,
        value_name: String,
        previous: Option<TypedRegValue>,
        applied: Option<TypedRegValue>,
    },
    RegistryKeyCreated {
        hive: RegHive,
        subkey: String,
    },
    ServiceStartMode {
        service_name: String,
        previous_start_type: u32,
        applied_start_type: u32,
    },
    PowerScheme {
        previous_guid: String,
        applied_guid: String,
    },
    PowerValueAc {
        scheme_guid: String,
        subgroup_guid: String,
        setting_guid: String,
        previous_val: u32,
        applied_val: u32,
    },
    FileBackup {
        original_path: String,
        backup_path: String,
        existed: bool,
    },
    DnsConfig {
        adapter_name: String,
        previous_servers: Vec<String>,
        was_dhcp: bool,
    },
    NtfsLastAccess {
        previous_val: Option<u32>,
        applied_val: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionJournal {
    pub journal_id: String,
    pub title: String,
    pub timestamp_iso: String,
    pub steps: Vec<TransactionStep>,
    pub rolled_back: bool,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VssHealth {
    pub service_exists: bool,
    pub start_type: u32,
    pub status_text: String,
    pub is_disabled: bool,
    pub system_drive: String,
    pub system_drive_free_bytes: u64,
    pub sufficient_space: bool,
    pub restore_enabled: Option<bool>,
    pub summary: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RestorePointInfo {
    pub sequence_number: u32,
    pub description: String,
    pub created_at: String,
    pub restore_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanerTargetSpec {
    pub base_path: String,
    pub wildcard: Option<String>,
    pub min_age_hours: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanerCategory {
    pub id: String,
    pub title: String,
    pub description: String,
    pub is_selected: bool,
    pub scanned_bytes: u64,
    pub file_count: usize,
    pub skipped_reason: Option<String>,
    pub warning_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloatAppItem {
    pub package_name: String,
    pub full_name: String,
    pub display_name: String,
    pub category: String,
    pub description: String,
    pub is_selected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PingState {
    NotTested,
    Measured { rtt_ms: u32, loss_pct: u8 },
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsProviderResult {
    pub id: String,
    pub name: String,
    pub primary_dns: String,
    pub secondary_dns: String,
    pub description: String,
    pub state: PingState,
    pub is_fastest: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiotRegionPing {
    pub region: String,
    pub host: String,
    pub rtt_ms: Option<u32>,
    pub method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameModeConfig {
    pub enable_p_core_pinning: bool,
    pub enable_high_priority: bool,
    pub enable_power_scheme: bool,
    pub enable_1ms_timer: bool,
    pub enable_background_throttle: bool,
    pub custom_targets: Vec<String>,
}

impl Default for GameModeConfig {
    fn default() -> Self {
        Self {
            enable_p_core_pinning: true,
            enable_high_priority: true,
            enable_power_scheme: true,
            enable_1ms_timer: true,
            enable_background_throttle: false,
            custom_targets: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameModeStatus {
    pub daemon_running: bool,
    pub active_game_name: Option<String>,
    pub active_game_pid: Option<u32>,
    pub is_anticheat_protected: bool,
    pub summary: String,
    pub current_timer_ms: f64,
    pub affinity_mask_hex: Option<String>,
    pub interventions: HashSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValorantPresetKind {
    CompetitivePotato,
    Tournament240Hz,
    LaptopBalanced,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValorantTopology {
    pub install_status: ValorantInstallStatus,
    pub install_path: Option<String>,
    pub riot_client_path: Option<String>,
    pub shipping_exe_path: Option<String>,
    pub config_paths: Vec<String>,
    pub active_config_path: Option<String>,
    pub resolution_quality: u32,
    pub vsync_disabled: bool,
    pub fso_disabled: bool,
    pub dpi_aware: bool,
    pub game_dvr_disabled: bool,
    pub nagle_disabled: bool,
    pub mmcss_optimized: bool,
    pub qos_policy_exists: bool,
    pub mouse_accel_disabled: bool,
    pub usb_suspend_disabled: bool,
    pub power_high_perf: bool,
    pub shader_cache_bytes: u64,
    pub log_cache_bytes: u64,
    pub crash_cache_bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmulatorTopology {
    pub bluestacks_installed: bool,
    pub bluestacks_version: Option<String>,
    pub bluestacks_conf: Option<String>,
    pub msi_installed: bool,
    pub msi_version: Option<String>,
    pub msi_conf: Option<String>,
    pub ldplayer_installed: bool,
    pub ldplayer_exe: Option<String>,
    pub ldplayer_configs: Vec<String>,
    pub instances_found: Vec<String>,
    pub fps_240_unlocked: bool,
    pub rog_profile_active: bool,
    pub astc_and_debloat_active: bool,
    pub ldplayer_optimized: bool,
    pub ifeo_priority_locked: bool,
    pub gpu_locked: bool,
    pub raw_mouse_active: bool,
    pub low_latency_net_active: bool,
    pub hypervisor_status: String,
    pub hypervisor_off: bool,
    pub hvci_disabled: bool,
    pub cache_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimePackageItem {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub is_installed: bool,
    pub detected_detail: Option<String>,
    pub is_selected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemReductConfig {
    pub clean_working_sets: bool,
    pub clean_system_file_cache: bool,
    pub clean_modified_page_list: bool,
    pub clean_standby_list: bool,
    pub clean_low_priority_standby: bool,
    pub clean_combine_memory_lists: bool,
    pub auto_reduct_enabled: bool,
    pub auto_reduct_threshold_pct: u32,
    pub auto_reduct_interval_mins: u32,
}

impl Default for MemReductConfig {
    fn default() -> Self {
        Self {
            clean_working_sets: true,
            clean_system_file_cache: true,
            clean_modified_page_list: true,
            clean_standby_list: true,
            clean_low_priority_standby: true,
            clean_combine_memory_lists: true,
            auto_reduct_enabled: false,
            auto_reduct_threshold_pct: 80,
            auto_reduct_interval_mins: 15,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemReductLiveStats {
    pub phys_total_bytes: u64,
    pub phys_used_bytes: u64,
    pub phys_free_bytes: u64,
    pub phys_load_pct: u32,
    pub commit_total_bytes: u64,
    pub commit_limit_bytes: u64,
    pub commit_free_bytes: u64,
    pub commit_load_pct: u32,
    pub system_cache_bytes: u64,
    pub kernel_paged_bytes: u64,
    pub kernel_nonpaged_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstalledAppKind {
    Win32x64,
    Win32x86,
    UserHkcu,
    StoreUwp,
}

impl InstalledAppKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Win32x64 => "Win32 (64-bit)",
            Self::Win32x86 => "Win32 (32-bit)",
            Self::UserHkcu => "User App",
            Self::StoreUwp => "Store / UWP",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledAppEntry {
    pub id: String,
    pub display_name: String,
    pub display_version: String,
    pub publisher: String,
    pub install_date: String,
    pub estimated_size_bytes: u64,
    pub install_location: Option<String>,
    pub uninstall_string: Option<String>,
    pub quiet_uninstall_string: Option<String>,
    pub registry_hive: Option<RegHive>,
    pub registry_subkey: String,
    pub is_win64_hive: bool,
    pub kind: InstalledAppKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeftoverKind {
    Directory,
    File,
    RegistryKey,
    RegistryValue,
}

impl LeftoverKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Directory => "FOLDER",
            Self::File => "FILE",
            Self::RegistryKey => "REG KEY",
            Self::RegistryValue => "REG VALUE",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeftoverItem {
    pub kind: LeftoverKind,
    pub display_path: String,
    pub fs_path: Option<String>,
    pub hive: Option<RegHive>,
    pub subkey: String,
    pub value_name: Option<String>,
    pub is_win64_hive: bool,
    pub size_bytes: u64,
    pub is_selected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: String,
    pub is_newer: bool,
    pub release_url: String,
    pub setup_download_url: String,
    pub release_notes: String,
}

#[derive(Debug, Clone)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate {
        version: String,
    },
    Available(UpdateCheckResult),
    Error(String),
}

impl Default for UpdateStatus {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub auto_restore_point_before_preset: bool,
    pub temp_cleaner_min_age_hours: u64,
    pub show_console_drawer: bool,
    #[serde(default)]
    pub mem_reduct: MemReductConfig,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_restore_point_before_preset: true,
            temp_cleaner_min_age_hours: 24,
            show_console_drawer: false,
            mem_reduct: MemReductConfig::default(),
        }
    }
}
