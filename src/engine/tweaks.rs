use crate::engine::network::icmp_ping_ipv4;
use crate::engine::run_command_timeout;
use crate::engine::safety::{
    reg_create_key_and_set_journaled, reg_delete_value_journaled, reg_set_journaled,
    reg_subkey_exists, read_reg_dword, read_reg_string, service_query_state,
    service_set_start_journaled,
};
use crate::models::{
    PresetId, RegHive, TransactionJournal, TransactionStep, TweakCategory, TweakOsTarget,
    TweakRisk, TweakViewItem, TypedRegValue,
};
use crate::system::gpu::read_pci_gpu_msi_states;
use crate::system::network::detect_adapters;
use crate::system::platform::{
    format_guid, get_active_power_scheme, parse_guid, read_power_scheme_name,
};
use std::ffi::c_void;
use windows::core::GUID;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Power::{
    PowerDuplicateScheme, PowerEnumerate, PowerReadACValueIndex, PowerSetActiveScheme,
    PowerWriteACValueIndex, ACCESS_SCHEME,
};
use windows::Win32::UI::WindowsAndMessaging::{
    SystemParametersInfoW, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE, SPI_SETMOUSE,
};
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};
use winreg::RegKey;

const WIN11_CLSID_SUBKEY: &str =
    r"Software\Classes\CLSID\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}";
const WIN11_CLSID_INPROC: &str =
    r"Software\Classes\CLSID\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}\InprocServer32";

pub const SUB_PROCESSOR_GUID: &str = "54533251-82be-4824-96c1-47b60b740d00";
pub const CPMINCORES_GUID: &str = "0cc5b647-c1df-4637-891a-dec35c318583";
pub const SUB_USB_GUID: &str = "2a737441-1930-4402-8d77-b2bebba308a3";
pub const USB_SUSPEND_GUID: &str = "48e6b7a6-50f5-4782-a5d4-53bb8f07e226";
const ULTIMATE_TEMPLATE_GUID: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";
const HIGH_PERF_GUID: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";
const BALANCED_GUID: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";

pub fn catalog_for_os(is_win11: bool) -> Vec<TweakViewItem> {
    let all = vec![
        tweak(
            "win11_classic_context_menu",
            "Classic Full Right-Click Context Menu",
            "Restores full Windows 10 context menu in Explorer without Shift+F10.",
            TweakCategory::WindowsUi,
            TweakRisk::Safe,
            TweakOsTarget::Win11Only,
            false,
            true,
            Some("Requires Explorer restart to take immediate visual effect."),
        ),
        tweak(
            "win11_taskbar_clean",
            "Declutter Taskbar Widgets, Chat & Copilot",
            "Hides taskbar Widgets feed, Chat, and Copilot buttons from taskbar.",
            TweakCategory::WindowsUi,
            TweakRisk::Safe,
            TweakOsTarget::Win11Only,
            false,
            false,
            None,
        ),
        tweak(
            "win11_explorer_clean",
            "Clean File Explorer Recent & Recommended",
            "Stops File Explorer Home from tracking/displaying recommended & recent clutter.",
            TweakCategory::WindowsUi,
            TweakRisk::Safe,
            TweakOsTarget::Win11Only,
            false,
            false,
            None,
        ),
        tweak(
            "win11_disable_copilot_recall",
            "Disable Windows 11 Copilot & AI Recall Snapshots",
            "Disables Windows Copilot sidebar, AI Recall screen analysis, and background AI indexing policies.",
            TweakCategory::Privacy,
            TweakRisk::Safe,
            TweakOsTarget::Win11Only,
            false,
            true,
            None,
        ),
        tweak(
            "win11_windowed_game_optimizations",
            "Enable Win11 Optimizations for Windowed Games (Flip-Model + VRR)",
            "Upgrades DX10/DX11 windowed games to low-latency DXGI flip-model presentation & Variable Refresh Rate.",
            TweakCategory::Gaming,
            TweakRisk::Safe,
            TweakOsTarget::Win11Only,
            false,
            false,
            None,
        ),
        tweak(
            "win11_disable_widgets_service",
            "Disable Windows 11 Widgets & WebExperience Background Host",
            "Blocks Dsh/AllowNewsAndInterests policy so Widgets.exe and WebView2 never run in background.",
            TweakCategory::WindowsUi,
            TweakRisk::Safe,
            TweakOsTarget::Win11Only,
            false,
            true,
            None,
        ),
        tweak(
            "win11_explorer_compact_gallery",
            "Win11 Explorer: Compact Mode, Open to This PC & Remove Gallery/OneDrive Ads",
            "Enables compact row spacing, opens Explorer to This PC, unpins Gallery, and disables sync provider ads.",
            TweakCategory::WindowsUi,
            TweakRisk::Safe,
            TweakOsTarget::Win11Only,
            false,
            true,
            None,
        ),
        tweak(
            "win11_disable_vbs_hvci",
            "Disable VBS & Memory Integrity (HVCI) for Maximum Gaming FPS",
            "Disables Virtualization-Based Security & HVCI hypervisor overhead (restores 5-15% CPU/gaming performance).",
            TweakCategory::Gaming,
            TweakRisk::Advanced,
            TweakOsTarget::Both,
            true,
            false,
            Some("Reboot required. Keep enabled if you rely on hypervisor code integrity in untrusted environments."),
        ),
        tweak(
            "win32_priority_separation_26",
            "Extreme Foreground Quantum Scheduling (Win32PrioritySeparation=0x26)",
            "Configures short variable CPU quantums with 3:1 foreground boost for ultra-crisp game frame pacing.",
            TweakCategory::Gaming,
            TweakRisk::Moderate,
            TweakOsTarget::Both,
            false,
            false,
            None,
        ),
        tweak(
            "svchost_split_ram_threshold",
            "Consolidate SvcHost Processes (High-RAM Threshold)",
            "Raises SvcHostSplitThresholdInKB so Windows consolidates background service hosts and reduces handle overhead.",
            TweakCategory::Speed,
            TweakRisk::Safe,
            TweakOsTarget::Both,
            true,
            false,
            None,
        ),
        tweak(
            "instant_ui_menu_responsiveness",
            "Zero Menu Delay & Fast App Shutdown (MenuShowDelay=0)",
            "Eliminates the 400ms desktop menu delay, sets instant taskbar thumbnail popups, and reduces app hang timeout.",
            TweakCategory::WindowsUi,
            TweakRisk::Safe,
            TweakOsTarget::Both,
            false,
            true,
            None,
        ),
        tweak(
            "disable_uwp_background_apps",
            "Disable Global Background UWP App Execution",
            "Prevents Store/UWP apps from waking in the background and consuming RAM/CPU while gaming.",
            TweakCategory::Privacy,
            TweakRisk::Safe,
            TweakOsTarget::Both,
            false,
            false,
            None,
        ),
        tweak(
            "disable_power_throttling",
            "Disable Windows CPU Power Throttling Policy",
            "Disables PowerThrottlingOff=1 so Windows never downclocks background audio/game worker threads.",
            TweakCategory::Speed,
            TweakRisk::Moderate,
            TweakOsTarget::Both,
            false,
            false,
            None,
        ),
        tweak(
            "win10_news_interests",
            "Disable Taskbar News & Interests",
            "Turns off Windows 10 taskbar news feed widget and background webView.",
            TweakCategory::WindowsUi,
            TweakRisk::Safe,
            TweakOsTarget::Win10Only,
            false,
            true,
            None,
        ),
        tweak(
            "win10_cortana_clean",
            "Hide Taskbar Cortana Button",
            "Removes the legacy Cortana button from the Windows 10 taskbar.",
            TweakCategory::WindowsUi,
            TweakRisk::Safe,
            TweakOsTarget::Win10Only,
            false,
            false,
            None,
        ),
        tweak(
            "win10_timeline_clean",
            "Disable Activity Feed & Timeline Upload",
            "Disables local activity history tracking and cloud synchronization.",
            TweakCategory::WindowsUi,
            TweakRisk::Safe,
            TweakOsTarget::Win10Only,
            false,
            false,
            None,
        ),
        tweak(
            "bing_search_start",
            "Disable Start Menu Web / Bing Search",
            "Keeps Windows Start Menu search strictly local and instantaneous.",
            TweakCategory::WindowsUi,
            TweakRisk::Safe,
            TweakOsTarget::Both,
            false,
            true,
            None,
        ),
        tweak(
            "consumer_features",
            "Block Silent Sponsored App Suggestions",
            "Prevents Windows ContentDeliveryManager from installing promoted Store apps.",
            TweakCategory::Privacy,
            TweakRisk::Safe,
            TweakOsTarget::Both,
            false,
            false,
            None,
        ),
        tweak(
            "telemetry_services",
            "Set Connected Telemetry (DiagTrack) to Manual",
            "Stops DiagTrack background upload daemon and sets startup to Manual.",
            TweakCategory::Privacy,
            TweakRisk::Safe,
            TweakOsTarget::Both,
            false,
            false,
            None,
        ),
        tweak(
            "telemetry_registry",
            "Minimize Diagnostic Telemetry Policy",
            "Sets DataCollection AllowTelemetry policy to 0 (Security/Basic depending on SKU).",
            TweakCategory::Privacy,
            TweakRisk::Safe,
            TweakOsTarget::Both,
            false,
            false,
            None,
        ),
        tweak(
            "advertising_id",
            "Disable Advertising ID & Tailored Tips",
            "Prevents per-user app advertising ID tracking and diagnostic tips.",
            TweakCategory::Privacy,
            TweakRisk::Safe,
            TweakOsTarget::Both,
            false,
            false,
            None,
        ),
        tweak(
            "delivery_optimization_p2p",
            "Disable Windows Update P2P Bandwidth Sharing",
            "Sets Delivery Optimization DODownloadMode=0 (HTTP only, no LAN/WAN upload).",
            TweakCategory::Privacy,
            TweakRisk::Safe,
            TweakOsTarget::Both,
            false,
            false,
            None,
        ),
        tweak(
            "game_dvr",
            "Disable Xbox GameDVR Background Capture",
            "Stops background screen recording hooks that cause D3D frame-time spikes.",
            TweakCategory::Gaming,
            TweakRisk::Safe,
            TweakOsTarget::Both,
            false,
            false,
            None,
        ),
        tweak(
            "game_mode_on",
            "Ensure Windows Game Mode Enabled",
            "Enables OS-level Game Mode scheduling prioritization during fullscreen play.",
            TweakCategory::Gaming,
            TweakRisk::Safe,
            TweakOsTarget::Both,
            false,
            false,
            None,
        ),
        tweak(
            "mouse_acceleration",
            "Disable Mouse Acceleration (1:1 Raw Input)",
            "Disables Enhance Pointer Precision in registry AND live via SystemParametersInfoW.",
            TweakCategory::Gaming,
            TweakRisk::Safe,
            TweakOsTarget::Both,
            false,
            false,
            None,
        ),
        tweak(
            "hags_scheduling",
            "Hardware-Accelerated GPU Scheduling (HAGS)",
            "Lets modern WDDM 2.7+ GPU drivers manage VRAM scheduling directly.",
            TweakCategory::Gaming,
            TweakRisk::Moderate,
            TweakOsTarget::Both,
            true,
            false,
            Some("Requires Windows restart after toggling."),
        ),
        tweak(
            "ultimate_performance_plan",
            "Activate Ultimate / High Performance Power Plan",
            "Reuses or activates Ultimate Performance via Win32 Power API without duplicates.",
            TweakCategory::Speed,
            TweakRisk::Safe,
            TweakOsTarget::Both,
            false,
            false,
            Some("On laptops on battery, prefer Balanced when unplugged for battery life."),
        ),
        tweak(
            "cpu_core_parking",
            "Unpark All CPU Cores on AC Power (CPMINCORES=100)",
            "Sets minimum unparked processor cores to 100% directly via PowerWriteACValueIndex.",
            TweakCategory::Speed,
            TweakRisk::Safe,
            TweakOsTarget::Both,
            false,
            false,
            None,
        ),
        tweak(
            "fast_ntfs_access",
            "Disable NTFS Last-Access Timestamp Updates",
            "Reduces metadata disk writes on SSDs via fsutil (masks bit-31 System Managed flag).",
            TweakCategory::Speed,
            TweakRisk::Safe,
            TweakOsTarget::Both,
            false,
            false,
            None,
        ),
        tweak(
            "usb_selective_suspend",
            "Disable USB Selective Suspend (AC Power)",
            "Prevents Windows from sleeping high-polling mice/audio USB hubs on AC.",
            TweakCategory::Speed,
            TweakRisk::Safe,
            TweakOsTarget::Both,
            false,
            false,
            None,
        ),
        tweak(
            "gpu_msi_mode",
            "Enable MSI Interrupts Strictly on Display GPUs",
            "Enables Message Signaled Interrupts ONLY on physical Display adapters (never touches NVMe/USB).",
            TweakCategory::Speed,
            TweakRisk::Advanced,
            TweakOsTarget::Both,
            true,
            false,
            Some("Display-class scoped & strictly journaled; reboot required."),
        ),
        tweak(
            "mmcss_network_throttle",
            "Disable MMCSS Network Throttling & Reserve",
            "Sets NetworkThrottlingIndex=0xFFFFFFFF & SystemResponsiveness=0 for low-jitter I/O.",
            TweakCategory::Network,
            TweakRisk::Moderate,
            TweakOsTarget::Both,
            false,
            false,
            None,
        ),
        tweak(
            "nagle_disable",
            "Disable TCP Nagle Algorithm on Primary NIC",
            "Sets TcpAckFrequency=1, TCPNoDelay=1, TcpDelAckTicks=0 on active physical NIC GUID.",
            TweakCategory::Network,
            TweakRisk::Moderate,
            TweakOsTarget::Both,
            false,
            false,
            None,
        ),
    ];

    all.into_iter()
        .filter(|t| match t.os_target {
            TweakOsTarget::Both => true,
            TweakOsTarget::Win11Only => is_win11,
            TweakOsTarget::Win10Only => !is_win11,
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn tweak(
    id: &str,
    title: &str,
    description: &str,
    category: TweakCategory,
    risk: TweakRisk,
    os_target: TweakOsTarget,
    requires_restart: bool,
    requires_explorer_restart: bool,
    note: Option<&str>,
) -> TweakViewItem {
    TweakViewItem {
        id: id.to_string(),
        title: title.to_string(),
        description: description.to_string(),
        category,
        risk,
        os_target,
        requires_restart,
        requires_explorer_restart,
        is_applied: false,
        is_selected: false,
        note: note.map(str::to_string),
    }
}

pub fn preset_ids(preset: PresetId) -> &'static [&'static str] {
    match preset {
        PresetId::EverydayBoost => &[
            "win11_classic_context_menu",
            "win11_taskbar_clean",
            "win11_explorer_clean",
            "win10_news_interests",
            "win10_cortana_clean",
            "win10_timeline_clean",
            "bing_search_start",
            "consumer_features",
            "telemetry_services",
            "telemetry_registry",
            "advertising_id",
            "delivery_optimization_p2p",
            "fast_ntfs_access",
        ],
        PresetId::GamingRig => &[
            "win11_classic_context_menu",
            "win11_taskbar_clean",
            "win11_explorer_clean",
            "win10_news_interests",
            "win10_cortana_clean",
            "win10_timeline_clean",
            "bing_search_start",
            "consumer_features",
            "telemetry_services",
            "telemetry_registry",
            "advertising_id",
            "delivery_optimization_p2p",
            "fast_ntfs_access",
            "game_dvr",
            "game_mode_on",
            "mouse_acceleration",
            "hags_scheduling",
            "ultimate_performance_plan",
            "cpu_core_parking",
            "usb_selective_suspend",
            "mmcss_network_throttle",
            "nagle_disable",
        ],
        PresetId::CompetitiveLowLatency => &[
            "win11_classic_context_menu",
            "win11_taskbar_clean",
            "win11_explorer_clean",
            "win10_news_interests",
            "win10_cortana_clean",
            "win10_timeline_clean",
            "bing_search_start",
            "consumer_features",
            "telemetry_services",
            "telemetry_registry",
            "advertising_id",
            "delivery_optimization_p2p",
            "fast_ntfs_access",
            "game_dvr",
            "game_mode_on",
            "mouse_acceleration",
            "hags_scheduling",
            "ultimate_performance_plan",
            "cpu_core_parking",
            "usb_selective_suspend",
            "mmcss_network_throttle",
            "nagle_disable",
            "gpu_msi_mode",
            "win32_priority_separation_26",
            "disable_power_throttling",
        ],
        PresetId::ExtremeWin11 => &[
            "win11_classic_context_menu",
            "win11_taskbar_clean",
            "win11_explorer_clean",
            "win11_disable_copilot_recall",
            "win11_windowed_game_optimizations",
            "win11_disable_widgets_service",
            "win11_explorer_compact_gallery",
            "win11_disable_vbs_hvci",
            "win32_priority_separation_26",
            "svchost_split_ram_threshold",
            "instant_ui_menu_responsiveness",
            "disable_uwp_background_apps",
            "disable_power_throttling",
            "win10_news_interests",
            "win10_cortana_clean",
            "win10_timeline_clean",
            "bing_search_start",
            "consumer_features",
            "telemetry_services",
            "telemetry_registry",
            "advertising_id",
            "delivery_optimization_p2p",
            "fast_ntfs_access",
            "game_dvr",
            "game_mode_on",
            "mouse_acceleration",
            "hags_scheduling",
            "ultimate_performance_plan",
            "cpu_core_parking",
            "usb_selective_suspend",
            "mmcss_network_throttle",
            "nagle_disable",
            "gpu_msi_mode",
        ],
    }
}

pub fn audit_tweaks(items: &mut [TweakViewItem]) {
    for item in items.iter_mut() {
        item.is_applied = check_tweak(&item.id);
    }
}

pub fn check_tweak(id: &str) -> bool {
    match id {
        "win11_classic_context_menu" => reg_subkey_exists(RegHive::Hkcu, WIN11_CLSID_INPROC),
        "win11_taskbar_clean" => {
            let sub = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
            read_reg_dword(RegHive::Hkcu, sub, "TaskbarDa") == Some(0)
                && read_reg_dword(RegHive::Hkcu, sub, "TaskbarMn") == Some(0)
                && read_reg_dword(RegHive::Hkcu, sub, "ShowCopilotButton") == Some(0)
        }
        "win11_explorer_clean" => {
            let sub = r"Software\Microsoft\Windows\CurrentVersion\Explorer";
            read_reg_dword(RegHive::Hkcu, sub, "ShowRecommendations") == Some(0)
                && read_reg_dword(RegHive::Hkcu, sub, "ShowRecent") == Some(0)
                && read_reg_dword(RegHive::Hkcu, sub, "ShowFrequent") == Some(0)
        }
        "win10_news_interests" => {
            read_reg_dword(
                RegHive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\Feeds",
                "ShellFeedsTaskbarViewMode",
            ) == Some(2)
        }
        "win10_cortana_clean" => {
            read_reg_dword(
                RegHive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced",
                "ShowCortanaButton",
            ) == Some(0)
        }
        "win10_timeline_clean" => {
            let sub = r"SOFTWARE\Policies\Microsoft\Windows\System";
            read_reg_dword(RegHive::Hklm, sub, "EnableActivityFeed") == Some(0)
                && read_reg_dword(RegHive::Hklm, sub, "PublishUserActivities") == Some(0)
                && read_reg_dword(RegHive::Hklm, sub, "UploadUserActivities") == Some(0)
        }
        "bing_search_start" => {
            read_reg_dword(
                RegHive::Hkcu,
                r"Software\Policies\Microsoft\Windows\Explorer",
                "DisableSearchBoxSuggestions",
            ) == Some(1)
                && read_reg_dword(
                    RegHive::Hkcu,
                    r"Software\Microsoft\Windows\CurrentVersion\Search",
                    "BingSearchEnabled",
                ) == Some(0)
        }
        "consumer_features" => {
            let sub = r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager";
            [
                "SystemPaneSuggestionsEnabled",
                "SilentInstalledAppsEnabled",
                "SoftLandingEnabled",
                "SubscribedContent-338388Enabled",
                "SubscribedContent-338389Enabled",
            ]
            .iter()
            .all(|v| read_reg_dword(RegHive::Hkcu, sub, v) == Some(0))
        }
        "telemetry_services" => match service_query_state("DiagTrack") {
            Ok((start_type, status)) => start_type >= 3 && status == "Stopped",
            Err(_) => false,
        },
        "telemetry_registry" => {
            read_reg_dword(
                RegHive::Hklm,
                r"SOFTWARE\Policies\Microsoft\Windows\DataCollection",
                "AllowTelemetry",
            ) == Some(0)
        }
        "advertising_id" => {
            read_reg_dword(
                RegHive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\AdvertisingInfo",
                "Enabled",
            ) == Some(0)
                && read_reg_dword(
                    RegHive::Hkcu,
                    r"Software\Microsoft\Windows\CurrentVersion\Privacy",
                    "TailoredExperiencesWithDiagnosticDataEnabled",
                ) == Some(0)
        }
        "delivery_optimization_p2p" => {
            read_reg_dword(
                RegHive::Hklm,
                r"SOFTWARE\Policies\Microsoft\Windows\DeliveryOptimization",
                "DODownloadMode",
            ) == Some(0)
        }
        "game_dvr" => {
            read_reg_dword(
                RegHive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\GameDVR",
                "AppCaptureEnabled",
            ) == Some(0)
                && read_reg_dword(RegHive::Hkcu, r"System\GameConfigStore", "GameDVR_Enabled") == Some(0)
        }
        "game_mode_on" => {
            read_reg_dword(RegHive::Hkcu, r"Software\Microsoft\GameBar", "AutoGameModeEnabled")
                != Some(0)
        }
        "mouse_acceleration" => {
            read_reg_string(RegHive::Hkcu, r"Control Panel\Mouse", "MouseSpeed").as_deref()
                == Some("0")
        }
        "hags_scheduling" => {
            read_reg_dword(
                RegHive::Hklm,
                r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers",
                "HwSchMode",
            ) == Some(2)
        }
        "ultimate_performance_plan" => check_is_high_or_ultimate_plan(),
        "cpu_core_parking" => read_active_power_ac_val(SUB_PROCESSOR_GUID, CPMINCORES_GUID) == Some(100),
        "fast_ntfs_access" => read_reg_dword(
            RegHive::Hklm,
            r"SYSTEM\CurrentControlSet\Control\FileSystem",
            "NtfsDisableLastAccessUpdate",
        )
        .map(decode_ntfs_last_access_is_disabled)
        .unwrap_or(false),
        "usb_selective_suspend" => read_active_power_ac_val(SUB_USB_GUID, USB_SUSPEND_GUID) == Some(0),
        "gpu_msi_mode" => {
            let states = read_pci_gpu_msi_states();
            !states.is_empty() && states.iter().all(|(_, s)| *s == Some(true))
        }
        "mmcss_network_throttle" => check_mmcss_optimized(),
        "nagle_disable" => check_nagle_disabled_on_primary(),
        "win11_disable_copilot_recall" => {
            read_reg_dword(
                RegHive::Hkcu,
                r"Software\Policies\Microsoft\Windows\WindowsCopilot",
                "TurnOffWindowsCopilot",
            ) == Some(1)
                && read_reg_dword(
                    RegHive::Hklm,
                    r"SOFTWARE\Policies\Microsoft\Windows\WindowsAI",
                    "DisableAIDataAnalysis",
                ) == Some(1)
        }
        "win11_windowed_game_optimizations" => {
            read_reg_string(
                RegHive::Hkcu,
                r"Software\Microsoft\DirectX\UserGpuPreferences",
                "DirectXUserGlobalSettings",
            )
            .map(|s| s.contains("SwapEffectUpgradeEnable=1"))
            .unwrap_or(false)
        }
        "win11_disable_widgets_service" => {
            read_reg_dword(
                RegHive::Hklm,
                r"SOFTWARE\Policies\Microsoft\Dsh",
                "AllowNewsAndInterests",
            ) == Some(0)
        }
        "win11_explorer_compact_gallery" => {
            let sub = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
            read_reg_dword(RegHive::Hkcu, sub, "UseCompactMode") == Some(1)
                && read_reg_dword(RegHive::Hkcu, sub, "LaunchTo") == Some(1)
                && read_reg_dword(RegHive::Hkcu, sub, "ShowSyncProviderNotifications") == Some(0)
        }
        "win11_disable_vbs_hvci" => {
            read_reg_dword(
                RegHive::Hklm,
                r"SYSTEM\CurrentControlSet\Control\DeviceGuard\Scenarios\HypervisorEnforcedCodeIntegrity",
                "Enabled",
            ) == Some(0)
        }
        "win32_priority_separation_26" => {
            read_reg_dword(
                RegHive::Hklm,
                r"SYSTEM\CurrentControlSet\Control\PriorityControl",
                "Win32PrioritySeparation",
            ) == Some(0x26)
        }
        "svchost_split_ram_threshold" => {
            read_reg_dword(
                RegHive::Hklm,
                r"SYSTEM\CurrentControlSet\Control",
                "SvcHostSplitThresholdInKB",
            )
            .map(|v| v >= 16_000_000)
            .unwrap_or(false)
        }
        "instant_ui_menu_responsiveness" => {
            read_reg_string(RegHive::Hkcu, r"Control Panel\Desktop", "MenuShowDelay").as_deref()
                == Some("0")
        }
        "disable_uwp_background_apps" => {
            read_reg_dword(
                RegHive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\BackgroundAccessApplications",
                "GlobalUserDisabled",
            ) == Some(1)
        }
        "disable_power_throttling" => {
            read_reg_dword(
                RegHive::Hklm,
                r"SYSTEM\CurrentControlSet\Control\Power\PowerThrottling",
                "PowerThrottlingOff",
            ) == Some(1)
        }
        _ => false,
    }
}

pub fn apply_tweak(id: &str, journal: &mut TransactionJournal) -> Result<(), String> {
    match id {
        "win11_classic_context_menu" => reg_create_key_and_set_journaled(
            journal,
            RegHive::Hkcu,
            WIN11_CLSID_SUBKEY,
            WIN11_CLSID_INPROC,
            "",
            TypedRegValue::String(String::new()),
        ),
        "win11_taskbar_clean" => {
            let sub = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
            reg_set_journaled(journal, RegHive::Hkcu, sub, "TaskbarDa", TypedRegValue::Dword(0))?;
            reg_set_journaled(journal, RegHive::Hkcu, sub, "TaskbarMn", TypedRegValue::Dword(0))?;
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                sub,
                "ShowCopilotButton",
                TypedRegValue::Dword(0),
            )
        }
        "win11_explorer_clean" => {
            let sub = r"Software\Microsoft\Windows\CurrentVersion\Explorer";
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                sub,
                "ShowRecommendations",
                TypedRegValue::Dword(0),
            )?;
            reg_set_journaled(journal, RegHive::Hkcu, sub, "ShowRecent", TypedRegValue::Dword(0))?;
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                sub,
                "ShowFrequent",
                TypedRegValue::Dword(0),
            )
        }
        "win10_news_interests" => reg_set_journaled(
            journal,
            RegHive::Hkcu,
            r"Software\Microsoft\Windows\CurrentVersion\Feeds",
            "ShellFeedsTaskbarViewMode",
            TypedRegValue::Dword(2),
        ),
        "win10_cortana_clean" => reg_set_journaled(
            journal,
            RegHive::Hkcu,
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced",
            "ShowCortanaButton",
            TypedRegValue::Dword(0),
        ),
        "win10_timeline_clean" => {
            let sub = r"SOFTWARE\Policies\Microsoft\Windows\System";
            reg_set_journaled(
                journal,
                RegHive::Hklm,
                sub,
                "EnableActivityFeed",
                TypedRegValue::Dword(0),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hklm,
                sub,
                "PublishUserActivities",
                TypedRegValue::Dword(0),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hklm,
                sub,
                "UploadUserActivities",
                TypedRegValue::Dword(0),
            )
        }
        "bing_search_start" => {
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Software\Policies\Microsoft\Windows\Explorer",
                "DisableSearchBoxSuggestions",
                TypedRegValue::Dword(1),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\Search",
                "BingSearchEnabled",
                TypedRegValue::Dword(0),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\Search",
                "CortanaConsent",
                TypedRegValue::Dword(0),
            )
        }
        "consumer_features" => {
            let sub = r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager";
            for val in [
                "SystemPaneSuggestionsEnabled",
                "SilentInstalledAppsEnabled",
                "SoftLandingEnabled",
                "SubscribedContent-338388Enabled",
                "SubscribedContent-338389Enabled",
            ] {
                reg_set_journaled(journal, RegHive::Hkcu, sub, val, TypedRegValue::Dword(0))?;
            }
            Ok(())
        }
        "telemetry_services" => service_set_start_journaled(journal, "DiagTrack", 3, true),
        "telemetry_registry" => reg_set_journaled(
            journal,
            RegHive::Hklm,
            r"SOFTWARE\Policies\Microsoft\Windows\DataCollection",
            "AllowTelemetry",
            TypedRegValue::Dword(0),
        ),
        "advertising_id" => {
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\AdvertisingInfo",
                "Enabled",
                TypedRegValue::Dword(0),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\Privacy",
                "TailoredExperiencesWithDiagnosticDataEnabled",
                TypedRegValue::Dword(0),
            )
        }
        "delivery_optimization_p2p" => reg_set_journaled(
            journal,
            RegHive::Hklm,
            r"SOFTWARE\Policies\Microsoft\Windows\DeliveryOptimization",
            "DODownloadMode",
            TypedRegValue::Dword(0),
        ),
        "game_dvr" => {
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\GameDVR",
                "AppCaptureEnabled",
                TypedRegValue::Dword(0),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"System\GameConfigStore",
                "GameDVR_Enabled",
                TypedRegValue::Dword(0),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hklm,
                r"SOFTWARE\Policies\Microsoft\Windows\GameDVR",
                "AllowGameDVR",
                TypedRegValue::Dword(0),
            )
        }
        "game_mode_on" => reg_set_journaled(
            journal,
            RegHive::Hkcu,
            r"Software\Microsoft\GameBar",
            "AutoGameModeEnabled",
            TypedRegValue::Dword(1),
        ),
        "mouse_acceleration" => apply_mouse_acceleration(journal, false),
        "hags_scheduling" => reg_set_journaled(
            journal,
            RegHive::Hklm,
            r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers",
            "HwSchMode",
            TypedRegValue::Dword(2),
        ),
        "ultimate_performance_plan" => activate_performance_power_plan(journal),
        "cpu_core_parking" => write_active_power_ac_val_journaled(
            journal,
            SUB_PROCESSOR_GUID,
            CPMINCORES_GUID,
            100,
        ),
        "fast_ntfs_access" => set_ntfs_last_access_journaled(journal, true),
        "usb_selective_suspend" => write_active_power_ac_val_journaled(
            journal,
            SUB_USB_GUID,
            USB_SUSPEND_GUID,
            0,
        ),
        "gpu_msi_mode" => set_display_gpus_msi_journaled(journal, true),
        "mmcss_network_throttle" => apply_mmcss_network_throttle(journal, true),
        "nagle_disable" => apply_nagle_on_primary(journal, true),
        "win11_disable_copilot_recall" => {
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Software\Policies\Microsoft\Windows\WindowsCopilot",
                "TurnOffWindowsCopilot",
                TypedRegValue::Dword(1),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hklm,
                r"SOFTWARE\Policies\Microsoft\Windows\WindowsCopilot",
                "TurnOffWindowsCopilot",
                TypedRegValue::Dword(1),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hklm,
                r"SOFTWARE\Policies\Microsoft\Windows\WindowsAI",
                "DisableAIDataAnalysis",
                TypedRegValue::Dword(1),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hklm,
                r"SOFTWARE\Policies\Microsoft\Windows\WindowsAI",
                "AllowRecallEnablement",
                TypedRegValue::Dword(0),
            )
        }
        "win11_windowed_game_optimizations" => reg_set_journaled(
            journal,
            RegHive::Hkcu,
            r"Software\Microsoft\DirectX\UserGpuPreferences",
            "DirectXUserGlobalSettings",
            TypedRegValue::String("VRROptimizeEnable=1;SwapEffectUpgradeEnable=1;".to_string()),
        ),
        "win11_disable_widgets_service" => {
            reg_set_journaled(
                journal,
                RegHive::Hklm,
                r"SOFTWARE\Policies\Microsoft\Dsh",
                "AllowNewsAndInterests",
                TypedRegValue::Dword(0),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced",
                "TaskbarDa",
                TypedRegValue::Dword(0),
            )
        }
        "win11_explorer_compact_gallery" => {
            let sub = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
            reg_set_journaled(journal, RegHive::Hkcu, sub, "UseCompactMode", TypedRegValue::Dword(1))?;
            reg_set_journaled(journal, RegHive::Hkcu, sub, "LaunchTo", TypedRegValue::Dword(1))?;
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                sub,
                "ShowSyncProviderNotifications",
                TypedRegValue::Dword(0),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Software\Classes\CLSID\{e88865ea-0e1c-4e20-9aa6-edcd0212c87c}",
                "System.IsPinnedToNameSpaceTree",
                TypedRegValue::Dword(0),
            )
        }
        "win11_disable_vbs_hvci" => {
            reg_set_journaled(
                journal,
                RegHive::Hklm,
                r"SYSTEM\CurrentControlSet\Control\DeviceGuard\Scenarios\HypervisorEnforcedCodeIntegrity",
                "Enabled",
                TypedRegValue::Dword(0),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hklm,
                r"SYSTEM\CurrentControlSet\Control\DeviceGuard",
                "EnableVirtualizationBasedSecurity",
                TypedRegValue::Dword(0),
            )
        }
        "win32_priority_separation_26" => reg_set_journaled(
            journal,
            RegHive::Hklm,
            r"SYSTEM\CurrentControlSet\Control\PriorityControl",
            "Win32PrioritySeparation",
            TypedRegValue::Dword(0x26),
        ),
        "svchost_split_ram_threshold" => reg_set_journaled(
            journal,
            RegHive::Hklm,
            r"SYSTEM\CurrentControlSet\Control",
            "SvcHostSplitThresholdInKB",
            TypedRegValue::Dword(67_108_864),
        ),
        "instant_ui_menu_responsiveness" => {
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Control Panel\Desktop",
                "MenuShowDelay",
                TypedRegValue::String("0".to_string()),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Control Panel\Desktop",
                "WaitToKillAppTimeout",
                TypedRegValue::String("2000".to_string()),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Control Panel\Desktop",
                "HungAppTimeout",
                TypedRegValue::String("1000".to_string()),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced",
                "ExtendedUIHoverTime",
                TypedRegValue::Dword(1),
            )
        }
        "disable_uwp_background_apps" => {
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\BackgroundAccessApplications",
                "GlobalUserDisabled",
                TypedRegValue::Dword(1),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hklm,
                r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
                "LetAppsRunInBackground",
                TypedRegValue::Dword(2),
            )
        }
        "disable_power_throttling" => reg_set_journaled(
            journal,
            RegHive::Hklm,
            r"SYSTEM\CurrentControlSet\Control\Power\PowerThrottling",
            "PowerThrottlingOff",
            TypedRegValue::Dword(1),
        ),
        _ => Err(format!("Unknown tweak ID: {id}")),
    }
}

pub fn revert_tweak(id: &str, journal: &mut TransactionJournal) -> Result<(), String> {
    match id {
        "win11_classic_context_menu" => {
            crate::engine::safety::delete_reg_tree(RegHive::Hkcu, WIN11_CLSID_SUBKEY)
        }
        "win11_taskbar_clean" => {
            let sub = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
            reg_set_journaled(journal, RegHive::Hkcu, sub, "TaskbarDa", TypedRegValue::Dword(1))?;
            reg_set_journaled(journal, RegHive::Hkcu, sub, "TaskbarMn", TypedRegValue::Dword(1))?;
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                sub,
                "ShowCopilotButton",
                TypedRegValue::Dword(1),
            )
        }
        "win11_explorer_clean" => {
            let sub = r"Software\Microsoft\Windows\CurrentVersion\Explorer";
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                sub,
                "ShowRecommendations",
                TypedRegValue::Dword(1),
            )?;
            reg_set_journaled(journal, RegHive::Hkcu, sub, "ShowRecent", TypedRegValue::Dword(1))?;
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                sub,
                "ShowFrequent",
                TypedRegValue::Dword(1),
            )
        }
        "win10_news_interests" => reg_set_journaled(
            journal,
            RegHive::Hkcu,
            r"Software\Microsoft\Windows\CurrentVersion\Feeds",
            "ShellFeedsTaskbarViewMode",
            TypedRegValue::Dword(0),
        ),
        "win10_cortana_clean" => reg_set_journaled(
            journal,
            RegHive::Hkcu,
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced",
            "ShowCortanaButton",
            TypedRegValue::Dword(1),
        ),
        "win10_timeline_clean" => {
            let sub = r"SOFTWARE\Policies\Microsoft\Windows\System";
            reg_delete_value_journaled(journal, RegHive::Hklm, sub, "EnableActivityFeed")?;
            reg_delete_value_journaled(journal, RegHive::Hklm, sub, "PublishUserActivities")?;
            reg_delete_value_journaled(journal, RegHive::Hklm, sub, "UploadUserActivities")
        }
        "bing_search_start" => {
            reg_delete_value_journaled(
                journal,
                RegHive::Hkcu,
                r"Software\Policies\Microsoft\Windows\Explorer",
                "DisableSearchBoxSuggestions",
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\Search",
                "BingSearchEnabled",
                TypedRegValue::Dword(1),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\Search",
                "CortanaConsent",
                TypedRegValue::Dword(1),
            )
        }
        "consumer_features" => {
            let sub = r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager";
            for val in [
                "SystemPaneSuggestionsEnabled",
                "SilentInstalledAppsEnabled",
                "SoftLandingEnabled",
                "SubscribedContent-338388Enabled",
                "SubscribedContent-338389Enabled",
            ] {
                reg_set_journaled(journal, RegHive::Hkcu, sub, val, TypedRegValue::Dword(1))?;
            }
            Ok(())
        }
        "telemetry_services" => service_set_start_journaled(journal, "DiagTrack", 2, false),
        "telemetry_registry" => reg_delete_value_journaled(
            journal,
            RegHive::Hklm,
            r"SOFTWARE\Policies\Microsoft\Windows\DataCollection",
            "AllowTelemetry",
        ),
        "advertising_id" => {
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\AdvertisingInfo",
                "Enabled",
                TypedRegValue::Dword(1),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\Privacy",
                "TailoredExperiencesWithDiagnosticDataEnabled",
                TypedRegValue::Dword(1),
            )
        }
        "delivery_optimization_p2p" => reg_delete_value_journaled(
            journal,
            RegHive::Hklm,
            r"SOFTWARE\Policies\Microsoft\Windows\DeliveryOptimization",
            "DODownloadMode",
        ),
        "game_dvr" => {
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\GameDVR",
                "AppCaptureEnabled",
                TypedRegValue::Dword(1),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"System\GameConfigStore",
                "GameDVR_Enabled",
                TypedRegValue::Dword(1),
            )?;
            reg_delete_value_journaled(
                journal,
                RegHive::Hklm,
                r"SOFTWARE\Policies\Microsoft\Windows\GameDVR",
                "AllowGameDVR",
            )
        }
        "game_mode_on" => reg_set_journaled(
            journal,
            RegHive::Hkcu,
            r"Software\Microsoft\GameBar",
            "AutoGameModeEnabled",
            TypedRegValue::Dword(1),
        ),
        "mouse_acceleration" => apply_mouse_acceleration(journal, true),
        "hags_scheduling" => reg_set_journaled(
            journal,
            RegHive::Hklm,
            r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers",
            "HwSchMode",
            TypedRegValue::Dword(1),
        ),
        "ultimate_performance_plan" => restore_balanced_power_plan(journal),
        "cpu_core_parking" => {
            write_active_power_ac_val_journaled(journal, SUB_PROCESSOR_GUID, CPMINCORES_GUID, 5)
        }
        "fast_ntfs_access" => set_ntfs_last_access_journaled(journal, false),
        "usb_selective_suspend" => {
            write_active_power_ac_val_journaled(journal, SUB_USB_GUID, USB_SUSPEND_GUID, 1)
        }
        "gpu_msi_mode" => set_display_gpus_msi_journaled(journal, false),
        "mmcss_network_throttle" => apply_mmcss_network_throttle(journal, false),
        "nagle_disable" => apply_nagle_on_primary(journal, false),
        "win11_disable_copilot_recall" => {
            reg_delete_value_journaled(
                journal,
                RegHive::Hkcu,
                r"Software\Policies\Microsoft\Windows\WindowsCopilot",
                "TurnOffWindowsCopilot",
            )?;
            reg_delete_value_journaled(
                journal,
                RegHive::Hklm,
                r"SOFTWARE\Policies\Microsoft\Windows\WindowsCopilot",
                "TurnOffWindowsCopilot",
            )?;
            reg_delete_value_journaled(
                journal,
                RegHive::Hklm,
                r"SOFTWARE\Policies\Microsoft\Windows\WindowsAI",
                "DisableAIDataAnalysis",
            )?;
            reg_delete_value_journaled(
                journal,
                RegHive::Hklm,
                r"SOFTWARE\Policies\Microsoft\Windows\WindowsAI",
                "AllowRecallEnablement",
            )
        }
        "win11_windowed_game_optimizations" => reg_set_journaled(
            journal,
            RegHive::Hkcu,
            r"Software\Microsoft\DirectX\UserGpuPreferences",
            "DirectXUserGlobalSettings",
            TypedRegValue::String("SwapEffectUpgradeEnable=0;".to_string()),
        ),
        "win11_disable_widgets_service" => {
            reg_delete_value_journaled(
                journal,
                RegHive::Hklm,
                r"SOFTWARE\Policies\Microsoft\Dsh",
                "AllowNewsAndInterests",
            )
        }
        "win11_explorer_compact_gallery" => {
            let sub = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
            reg_set_journaled(journal, RegHive::Hkcu, sub, "UseCompactMode", TypedRegValue::Dword(0))?;
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                sub,
                "ShowSyncProviderNotifications",
                TypedRegValue::Dword(1),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Software\Classes\CLSID\{e88865ea-0e1c-4e20-9aa6-edcd0212c87c}",
                "System.IsPinnedToNameSpaceTree",
                TypedRegValue::Dword(1),
            )
        }
        "win11_disable_vbs_hvci" => {
            reg_set_journaled(
                journal,
                RegHive::Hklm,
                r"SYSTEM\CurrentControlSet\Control\DeviceGuard\Scenarios\HypervisorEnforcedCodeIntegrity",
                "Enabled",
                TypedRegValue::Dword(1),
            )?;
            reg_set_journaled(
                journal,
                RegHive::Hklm,
                r"SYSTEM\CurrentControlSet\Control\DeviceGuard",
                "EnableVirtualizationBasedSecurity",
                TypedRegValue::Dword(1),
            )
        }
        "win32_priority_separation_26" => reg_set_journaled(
            journal,
            RegHive::Hklm,
            r"SYSTEM\CurrentControlSet\Control\PriorityControl",
            "Win32PrioritySeparation",
            TypedRegValue::Dword(0x02),
        ),
        "svchost_split_ram_threshold" => reg_set_journaled(
            journal,
            RegHive::Hklm,
            r"SYSTEM\CurrentControlSet\Control",
            "SvcHostSplitThresholdInKB",
            TypedRegValue::Dword(0x0038_0000),
        ),
        "instant_ui_menu_responsiveness" => {
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Control Panel\Desktop",
                "MenuShowDelay",
                TypedRegValue::String("400".to_string()),
            )?;
            reg_delete_value_journaled(
                journal,
                RegHive::Hkcu,
                r"Control Panel\Desktop",
                "WaitToKillAppTimeout",
            )?;
            reg_delete_value_journaled(
                journal,
                RegHive::Hkcu,
                r"Control Panel\Desktop",
                "HungAppTimeout",
            )?;
            reg_delete_value_journaled(
                journal,
                RegHive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced",
                "ExtendedUIHoverTime",
            )
        }
        "disable_uwp_background_apps" => {
            reg_set_journaled(
                journal,
                RegHive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\BackgroundAccessApplications",
                "GlobalUserDisabled",
                TypedRegValue::Dword(0),
            )?;
            reg_delete_value_journaled(
                journal,
                RegHive::Hklm,
                r"SOFTWARE\Policies\Microsoft\Windows\AppPrivacy",
                "LetAppsRunInBackground",
            )
        }
        "disable_power_throttling" => reg_delete_value_journaled(
            journal,
            RegHive::Hklm,
            r"SYSTEM\CurrentControlSet\Control\Power\PowerThrottling",
            "PowerThrottlingOff",
        ),
        _ => Err(format!("Unknown tweak ID: {id}")),
    }
}

pub fn apply_mouse_acceleration(
    journal: &mut TransactionJournal,
    enable_accel: bool,
) -> Result<(), String> {
    let (s, t1, t2) = if enable_accel {
        ("1", "6", "10")
    } else {
        ("0", "0", "0")
    };
    let sub = r"Control Panel\Mouse";
    reg_set_journaled(
        journal,
        RegHive::Hkcu,
        sub,
        "MouseSpeed",
        TypedRegValue::String(s.to_string()),
    )?;
    reg_set_journaled(
        journal,
        RegHive::Hkcu,
        sub,
        "MouseThreshold1",
        TypedRegValue::String(t1.to_string()),
    )?;
    reg_set_journaled(
        journal,
        RegHive::Hkcu,
        sub,
        "MouseThreshold2",
        TypedRegValue::String(t2.to_string()),
    )?;

    let mut params = if enable_accel {
        [6i32, 10i32, 1i32]
    } else {
        [0i32, 0i32, 0i32]
    };
    unsafe {
        let flags = SPIF_UPDATEINIFILE | SPIF_SENDCHANGE;
        SystemParametersInfoW(
            SPI_SETMOUSE,
            0,
            Some(params.as_mut_ptr() as *mut c_void),
            flags,
        )
        .map_err(|e| format!("SystemParametersInfoW(SPI_SETMOUSE) failed: {e}"))?;
    }
    Ok(())
}

pub fn check_is_high_or_ultimate_plan() -> bool {
    let Some((guid, name)) = get_active_power_scheme() else {
        return false;
    };
    let n_low = name.to_ascii_lowercase();
    guid.eq_ignore_ascii_case(HIGH_PERF_GUID)
        || guid.eq_ignore_ascii_case(ULTIMATE_TEMPLATE_GUID)
        || n_low.contains("ultimate performance")
        || n_low.contains("high performance")
}

pub fn enumerate_power_schemes() -> Vec<(GUID, String)> {
    let mut schemes = Vec::new();
    unsafe {
        let mut idx = 0u32;
        loop {
            let mut guid = GUID::default();
            let mut size = std::mem::size_of::<GUID>() as u32;
            let err = PowerEnumerate(
                None,
                None,
                None,
                ACCESS_SCHEME,
                idx,
                Some(&mut guid as *mut GUID as *mut u8),
                &mut size,
            );
            if err != ERROR_SUCCESS {
                break;
            }
            let name = read_power_scheme_name(&guid).unwrap_or_else(|| format_guid(&guid));
            schemes.push((guid, name));
            idx += 1;
        }
    }
    schemes
}

pub fn activate_performance_power_plan(journal: &mut TransactionJournal) -> Result<(), String> {
    let prev_guid = get_active_power_scheme()
        .map(|(g, _)| g)
        .unwrap_or_else(|| BALANCED_GUID.to_string());

    let target_guid = ensure_or_find_performance_scheme_guid()?;
    let target_str = format_guid(&target_guid);

    let err = unsafe { PowerSetActiveScheme(None, Some(&target_guid)) };
    if err != ERROR_SUCCESS {
        return Err(format!("PowerSetActiveScheme failed with code {}", err.0));
    }

    if !prev_guid.eq_ignore_ascii_case(&target_str) {
        journal.steps.push(TransactionStep::PowerScheme {
            previous_guid: prev_guid,
            applied_guid: target_str,
        });
    }
    Ok(())
}

pub fn ensure_or_find_performance_scheme_guid() -> Result<GUID, String> {
    let schemes = enumerate_power_schemes();
    if let Some((g, _)) = schemes
        .iter()
        .find(|(_, name)| name.to_ascii_lowercase().contains("ultimate performance"))
    {
        return Ok(*g);
    }

    // Duplicate Ultimate Performance once (no duplicate spam!)
    if let Some(template) = parse_guid(ULTIMATE_TEMPLATE_GUID) {
        unsafe {
            let mut dest_ptr: *mut GUID = std::ptr::null_mut();
            let err = PowerDuplicateScheme(None, &template, &mut dest_ptr);
            if err == ERROR_SUCCESS && !dest_ptr.is_null() {
                let new_guid = *dest_ptr;
                let _ = windows::Win32::Foundation::LocalFree(
                    windows::Win32::Foundation::HLOCAL(dest_ptr as _),
                );
                return Ok(new_guid);
            }
        }
    }

    if let Some((g, _)) = schemes
        .iter()
        .find(|(_, name)| name.to_ascii_lowercase().contains("high performance"))
    {
        return Ok(*g);
    }

    if let Some(hp) = parse_guid(HIGH_PERF_GUID) {
        return Ok(hp);
    }

    Err("No Ultimate or High Performance power scheme available (Modern Standby S0ix restriction)".to_string())
}

fn restore_balanced_power_plan(journal: &mut TransactionJournal) -> Result<(), String> {
    let prev_guid = get_active_power_scheme()
        .map(|(g, _)| g)
        .unwrap_or_default();
    let bal = parse_guid(BALANCED_GUID).ok_or_else(|| "Invalid Balanced GUID".to_string())?;
    let err = unsafe { PowerSetActiveScheme(None, Some(&bal)) };
    if err != ERROR_SUCCESS {
        return Err(format!("Failed restoring Balanced power plan ({})", err.0));
    }
    if !prev_guid.is_empty() && !prev_guid.eq_ignore_ascii_case(BALANCED_GUID) {
        journal.steps.push(TransactionStep::PowerScheme {
            previous_guid: prev_guid,
            applied_guid: BALANCED_GUID.to_string(),
        });
    }
    Ok(())
}

pub fn read_active_power_ac_val(subgroup_str: &str, setting_str: &str) -> Option<u32> {
    let (active_str, _) = get_active_power_scheme()?;
    let active = parse_guid(&active_str)?;
    let sub = parse_guid(subgroup_str)?;
    let set = parse_guid(setting_str)?;
    let mut val = 0u32;
    let err = unsafe { PowerReadACValueIndex(None, Some(&active), Some(&sub), Some(&set), &mut val) };
    if err == ERROR_SUCCESS {
        Some(val)
    } else {
        None
    }
}

pub fn write_active_power_ac_val_journaled(
    journal: &mut TransactionJournal,
    subgroup_str: &str,
    setting_str: &str,
    new_val: u32,
) -> Result<(), String> {
    let (active_str, _) =
        get_active_power_scheme().ok_or_else(|| "No active power scheme".to_string())?;
    let active = parse_guid(&active_str).ok_or_else(|| "Bad active GUID".to_string())?;
    let sub = parse_guid(subgroup_str).ok_or_else(|| "Bad subgroup GUID".to_string())?;
    let set = parse_guid(setting_str).ok_or_else(|| "Bad setting GUID".to_string())?;

    let mut prev = 0u32;
    let read_err =
        unsafe { PowerReadACValueIndex(None, Some(&active), Some(&sub), Some(&set), &mut prev) };
    if read_err != ERROR_SUCCESS {
        return Err(format!(
            "PowerReadACValueIndex failed with code {}",
            read_err.0
        ));
    }

    let write_err =
        unsafe { PowerWriteACValueIndex(None, &active, Some(&sub), Some(&set), new_val) };
    if write_err != ERROR_SUCCESS {
        return Err(format!(
            "PowerWriteACValueIndex failed with code {}",
            write_err.0
        ));
    }
    let _ = unsafe { PowerSetActiveScheme(None, Some(&active)) };

    journal.steps.push(TransactionStep::PowerValueAc {
        scheme_guid: active_str,
        subgroup_guid: subgroup_str.to_string(),
        setting_guid: setting_str.to_string(),
        previous_val: prev,
        applied_val: new_val,
    });
    Ok(())
}

/// Bit 31 (0x80000000) indicates "System Managed". Lower 2 bits hold mode:
/// 0 = User Managed Enabled, 1 = User Managed Disabled, 2 = System Managed Disabled, 3 = System Managed Enabled.
pub fn decode_ntfs_last_access_is_disabled(reg_val: u32) -> bool {
    let mode = reg_val & 0x7FFF_FFFF;
    mode == 1 || mode == 2
}

fn set_ntfs_last_access_journaled(
    journal: &mut TransactionJournal,
    disable: bool,
) -> Result<(), String> {
    let prev = read_reg_dword(
        RegHive::Hklm,
        r"SYSTEM\CurrentControlSet\Control\FileSystem",
        "NtfsDisableLastAccessUpdate",
    );
    let target_mode = if disable { 1u32 } else { 2u32 };
    let arg = target_mode.to_string();
    let out = run_command_timeout(
        "fsutil.exe",
        &["behavior", "set", "disablelastaccess", &arg],
        5000,
    )?;
    if out.exit_code != 0 {
        return Err(format!(
            "fsutil behavior set disablelastaccess {arg} failed: {}",
            out.stderr.trim()
        ));
    }
    journal.steps.push(TransactionStep::NtfsLastAccess {
        previous_val: prev,
        applied_val: target_mode,
    });
    Ok(())
}

/// Strictly targets ONLY PCI devices matching Display adapter ClassGUID `{4d36e968-e325-11ce-bfc1-08002be10318}`.
fn set_display_gpus_msi_journaled(
    journal: &mut TransactionJournal,
    enable: bool,
) -> Result<(), String> {
    let gpus = read_pci_gpu_msi_states();
    if gpus.is_empty() {
        return Err("No PCI Display adapters discovered in Enum\\PCI".to_string());
    }
    let val = if enable { 1u32 } else { 0u32 };
    let mut updated = 0usize;
    for (pnp, _) in gpus {
        let sub = format!(
            r"SYSTEM\CurrentControlSet\Enum\{}\Device Parameters\Interrupt Management\MessageSignaledInterruptProperties",
            pnp
        );
        reg_set_journaled(journal, RegHive::Hklm, &sub, "MSISupported", TypedRegValue::Dword(val))?;
        updated += 1;
    }
    if updated == 0 {
        return Err("No GPU MSI registry targets updated".to_string());
    }
    Ok(())
}

pub fn check_mmcss_optimized() -> bool {
    let sub = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile";
    read_reg_dword(RegHive::Hklm, sub, "NetworkThrottlingIndex") == Some(u32::MAX)
        && read_reg_dword(RegHive::Hklm, sub, "SystemResponsiveness") == Some(0)
}

pub fn apply_mmcss_network_throttle(
    journal: &mut TransactionJournal,
    optimize: bool,
) -> Result<(), String> {
    let sub = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile";
    if optimize {
        reg_set_journaled(
            journal,
            RegHive::Hklm,
            sub,
            "NetworkThrottlingIndex",
            TypedRegValue::Dword(u32::MAX),
        )?;
        reg_set_journaled(
            journal,
            RegHive::Hklm,
            sub,
            "SystemResponsiveness",
            TypedRegValue::Dword(0),
        )
    } else {
        reg_set_journaled(
            journal,
            RegHive::Hklm,
            sub,
            "NetworkThrottlingIndex",
            TypedRegValue::Dword(10),
        )?;
        reg_set_journaled(
            journal,
            RegHive::Hklm,
            sub,
            "SystemResponsiveness",
            TypedRegValue::Dword(20),
        )
    }
}

pub fn check_nagle_disabled_on_primary() -> bool {
    let mut diag = Vec::new();
    let adapters = detect_adapters(&mut diag);
    let Some(primary) = adapters
        .iter()
        .find(|a| a.is_physical_primary)
        .or_else(|| adapters.first())
    else {
        return false;
    };
    if primary.guid.is_empty() {
        return false;
    }
    let sub = format!(
        r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces\{}",
        primary.guid
    );
    read_reg_dword(RegHive::Hklm, &sub, "TcpAckFrequency") == Some(1)
        && read_reg_dword(RegHive::Hklm, &sub, "TCPNoDelay") == Some(1)
}

pub fn apply_nagle_on_primary(
    journal: &mut TransactionJournal,
    disable_nagle: bool,
) -> Result<(), String> {
    let mut diag = Vec::new();
    let adapters = detect_adapters(&mut diag);
    let primary = adapters
        .iter()
        .find(|a| a.is_physical_primary)
        .or_else(|| adapters.first())
        .ok_or_else(|| "No active physical network adapter detected".to_string())?;

    if primary.guid.is_empty() {
        return Err("Primary adapter missing interface GUID".to_string());
    }

    let sub = format!(
        r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces\{}",
        primary.guid
    );

    if !reg_subkey_exists(RegHive::Hklm, &sub) {
        return Err(format!("Tcpip interface registry key missing for {}", primary.guid));
    }

    if disable_nagle {
        reg_set_journaled(
            journal,
            RegHive::Hklm,
            &sub,
            "TcpAckFrequency",
            TypedRegValue::Dword(1),
        )?;
        reg_set_journaled(
            journal,
            RegHive::Hklm,
            &sub,
            "TCPNoDelay",
            TypedRegValue::Dword(1),
        )?;
        reg_set_journaled(
            journal,
            RegHive::Hklm,
            &sub,
            "TcpDelAckTicks",
            TypedRegValue::Dword(0),
        )
    } else {
        reg_delete_value_journaled(journal, RegHive::Hklm, &sub, "TcpAckFrequency")?;
        reg_delete_value_journaled(journal, RegHive::Hklm, &sub, "TCPNoDelay")?;
        reg_delete_value_journaled(journal, RegHive::Hklm, &sub, "TcpDelAckTicks")
    }
}

pub fn restart_explorer() -> Result<(), String> {
    let _ = run_command_timeout("taskkill.exe", &["/F", "/IM", "explorer.exe"], 4000);
    std::process::Command::new("explorer.exe")
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed restarting explorer.exe: {e}"))
}

#[allow(dead_code)]
fn _unused_import_guard() {
    let _ = (HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY, RegKey::predef, icmp_ping_ipv4);
}

#[cfg(test)]
mod tests {
    use super::decode_ntfs_last_access_is_disabled;

    #[test]
    fn ntfs_last_access_bitmask_tests() {
        // 0 = User Managed Enabled (timestamps ON -> not optimized)
        assert!(!decode_ntfs_last_access_is_disabled(0));
        // 1 = User Managed Disabled (optimized)
        assert!(decode_ntfs_last_access_is_disabled(1));
        // 2 | 0x80000000 = System Managed Disabled (Win10/11 large volume default -> optimized!)
        assert!(decode_ntfs_last_access_is_disabled(0x8000_0002));
        // 3 | 0x80000000 = System Managed Enabled (timestamps ON -> not optimized!)
        assert!(!decode_ntfs_last_access_is_disabled(0x8000_0003));
        assert!(!decode_ntfs_last_access_is_disabled(3));
    }
}
