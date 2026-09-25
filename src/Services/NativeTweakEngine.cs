using System;
using System.Diagnostics;
using System.IO;
using System.ServiceProcess;
using System.Text;
using System.Text.RegularExpressions;
using Microsoft.Win32;
using AnxiouslyOptimized.Models;

namespace AnxiouslyOptimized.Services
{
    public static class NativeTweakEngine
    {
        public static TransactionJournal ActiveJournal = null;

        #region Registry Helper Methods
        public static int GetDword(RegistryHive hive, string subKey, string valueName, int defaultValue = -1)
        {
            try
            {
                using (var baseKey = RegistryKey.OpenBaseKey(hive, RegistryView.Registry64))
                using (var key = baseKey.OpenSubKey(subKey, false))
                {
                    if (key == null) return defaultValue;
                    object val = key.GetValue(valueName);
                    if (val is int) return (int)val;
                    if (val != null)
                    {
                        int parsed;
                        if (int.TryParse(val.ToString(), out parsed)) return parsed;
                    }
                }
            }
            catch { }
            return defaultValue;
        }

        public static bool SetDword(RegistryHive hive, string subKey, string valueName, int value)
        {
            try
            {
                if (ActiveJournal != null)
                {
                    int prev = GetDword(hive, subKey, valueName, -1);
                    string target = (hive == RegistryHive.LocalMachine ? "HKLM\\" : "HKCU\\") + subKey;
                    SafetyService.RecordRegistryChange(ActiveJournal, target, valueName, prev == -1 ? null : (object)prev, prev == -1 ? RegistryValueKind.None : RegistryValueKind.DWord, value);
                }

                using (var baseKey = RegistryKey.OpenBaseKey(hive, RegistryView.Registry64))
                using (var key = baseKey.CreateSubKey(subKey))
                {
                    if (key != null)
                    {
                        key.SetValue(valueName, value, RegistryValueKind.DWord);
                        return true;
                    }
                }
            }
            catch { }
            return false;
        }

        public static string GetString(RegistryHive hive, string subKey, string valueName, string defaultValue = null)
        {
            try
            {
                using (var baseKey = RegistryKey.OpenBaseKey(hive, RegistryView.Registry64))
                using (var key = baseKey.OpenSubKey(subKey, false))
                {
                    if (key == null) return defaultValue;
                    object val = key.GetValue(valueName);
                    return val != null ? val.ToString() : defaultValue;
                }
            }
            catch { }
            return defaultValue;
        }

        public static bool SetString(RegistryHive hive, string subKey, string valueName, string value)
        {
            try
            {
                if (ActiveJournal != null)
                {
                    string prev = GetString(hive, subKey, valueName, null);
                    string target = (hive == RegistryHive.LocalMachine ? "HKLM\\" : "HKCU\\") + subKey;
                    SafetyService.RecordRegistryChange(ActiveJournal, target, valueName, prev, prev == null ? RegistryValueKind.None : RegistryValueKind.String, value);
                }

                using (var baseKey = RegistryKey.OpenBaseKey(hive, RegistryView.Registry64))
                using (var key = baseKey.CreateSubKey(subKey))
                {
                    if (key != null)
                    {
                        key.SetValue(valueName, value ?? string.Empty, RegistryValueKind.String);
                        return true;
                    }
                }
            }
            catch { }
            return false;
        }

        public static bool DeleteValue(RegistryHive hive, string subKey, string valueName)
        {
            try
            {
                using (var baseKey = RegistryKey.OpenBaseKey(hive, RegistryView.Registry64))
                using (var key = baseKey.OpenSubKey(subKey, true))
                {
                    if (key != null)
                    {
                        key.DeleteValue(valueName, false);
                        return true;
                    }
                }
            }
            catch { }
            return false;
        }

        public static bool KeyExists(RegistryHive hive, string subKey)
        {
            try
            {
                using (var baseKey = RegistryKey.OpenBaseKey(hive, RegistryView.Registry64))
                using (var key = baseKey.OpenSubKey(subKey, false))
                {
                    return key != null;
                }
            }
            catch { }
            return false;
        }

        public static bool DeleteSubKeyTree(RegistryHive hive, string subKey)
        {
            try
            {
                using (var baseKey = RegistryKey.OpenBaseKey(hive, RegistryView.Registry64))
                {
                    baseKey.DeleteSubKeyTree(subKey, false);
                    return true;
                }
            }
            catch { }
            return false;
        }
        #endregion

        #region Windows Service Management
        public static int GetServiceStartMode(string serviceName)
        {
            return GetDword(RegistryHive.LocalMachine, @"SYSTEM\CurrentControlSet\Services\" + serviceName, "Start", -1);
        }

        public static bool SetServiceStartMode(string serviceName, int startMode)
        {
            if (ActiveJournal != null)
            {
                int prev = GetServiceStartMode(serviceName);
                SafetyService.RecordServiceChange(ActiveJournal, serviceName, prev, startMode);
            }
            return SetDword(RegistryHive.LocalMachine, @"SYSTEM\CurrentControlSet\Services\" + serviceName, "Start", startMode);
        }

        public static void StopServiceNative(string serviceName)
        {
            try
            {
                using (var sc = new ServiceController(serviceName))
                {
                    if (sc.Status == ServiceControllerStatus.Running || sc.Status == ServiceControllerStatus.StartPending)
                    {
                        sc.Stop();
                        sc.WaitForStatus(ServiceControllerStatus.Stopped, TimeSpan.FromMilliseconds(2000));
                    }
                }
            }
            catch { }
        }

        public static void StartServiceNative(string serviceName)
        {
            try
            {
                using (var sc = new ServiceController(serviceName))
                {
                    if (sc.Status == ServiceControllerStatus.Stopped || sc.Status == ServiceControllerStatus.StopPending)
                    {
                        sc.Start();
                    }
                }
            }
            catch { }
        }
        #endregion

        #region Native Tweak Dispatcher
        public static bool CanHandleNatively(string id)
        {
            switch (id)
            {
                case "win11_classic_context_menu":
                case "win11_taskbar_clean":
                case "win11_explorer_clean":
                case "win10_news_interests":
                case "win10_cortana_clean":
                case "win10_timeline_clean":
                case "bing_search_start":
                case "consumer_features":
                case "telemetry_services":
                case "telemetry_registry":
                case "advertising_id":
                case "game_dvr":
                case "mouse_acceleration":
                case "ultimate_performance_plan":
                case "hags_scheduling":
                case "network_autotuning":
                case "fast_ntfs_access":
                case "emulator_bluestacks_fps":
                case "cpu_core_parking":
                case "delivery_optimization_p2p":
                case "gpu_msi_mode":
                    return true;
                default:
                    return false;
            }
        }

        public static bool CheckTweakNative(string id, out bool isApplied)
        {
            isApplied = false;
            switch (id)
            {
                case "win11_classic_context_menu":
                    isApplied = KeyExists(RegistryHive.CurrentUser, @"Software\Classes\CLSID\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}\InprocServer32");
                    return true;

                case "win11_taskbar_clean":
                    isApplied = GetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced", "TaskbarDa", -1) == 0;
                    return true;

                case "win11_explorer_clean":
                    isApplied = GetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Explorer", "ShowRecommendations", -1) == 0;
                    return true;

                case "win10_news_interests":
                    isApplied = GetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Feeds", "ShellFeedsTaskbarViewMode", -1) == 2;
                    return true;

                case "win10_cortana_clean":
                    isApplied = GetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced", "ShowCortanaButton", -1) == 0;
                    return true;

                case "win10_timeline_clean":
                    isApplied = GetDword(RegistryHive.LocalMachine, @"SOFTWARE\Policies\Microsoft\Windows\System", "EnableActivityFeed", -1) == 0;
                    return true;

                case "bing_search_start":
                    isApplied = GetDword(RegistryHive.CurrentUser, @"Software\Policies\Microsoft\Windows\Explorer", "DisableSearchBoxSuggestions", -1) == 1;
                    return true;

                case "consumer_features":
                    isApplied = GetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager", "SystemPaneSuggestionsEnabled", -1) == 0;
                    return true;

                case "telemetry_services":
                    int diagStart = GetServiceStartMode("DiagTrack");
                    isApplied = (diagStart == 3 || diagStart == 4);
                    return true;

                case "telemetry_registry":
                    isApplied = GetDword(RegistryHive.LocalMachine, @"SOFTWARE\Policies\Microsoft\Windows\DataCollection", "AllowTelemetry", -1) == 0;
                    return true;

                case "advertising_id":
                    isApplied = GetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\AdvertisingInfo", "Enabled", -1) == 0;
                    return true;

                case "game_dvr":
                    isApplied = GetDword(RegistryHive.CurrentUser, @"SOFTWARE\Microsoft\Windows\CurrentVersion\GameDVR", "AppCaptureEnabled", -1) == 0;
                    return true;

                case "mouse_acceleration":
                    string speed = GetString(RegistryHive.CurrentUser, @"Control Panel\Mouse", "MouseSpeed", "1");
                    isApplied = (speed == "0");
                    return true;

                case "ultimate_performance_plan":
                    string planOutput = RunToolSilent("powercfg.exe", "/getactivescheme");
                    isApplied = (planOutput.IndexOf("Ultimate", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                 planOutput.IndexOf("High", StringComparison.OrdinalIgnoreCase) >= 0);
                    return true;

                case "hags_scheduling":
                    isApplied = GetDword(RegistryHive.LocalMachine, @"SYSTEM\CurrentControlSet\Control\GraphicsDrivers", "HwSchMode", -1) == 2;
                    return true;

                case "network_autotuning":
                    string netshOutput = RunToolSilent("netsh.exe", "int tcp show global");
                    isApplied = netshOutput.IndexOf("normal", StringComparison.OrdinalIgnoreCase) >= 0;
                    return true;

                case "fast_ntfs_access":
                    int ntfsAccess = GetDword(RegistryHive.LocalMachine, @"SYSTEM\CurrentControlSet\Control\FileSystem", "NtfsDisableLastAccessUpdate", -1);
                    isApplied = (ntfsAccess == 1 || ntfsAccess == 3);
                    return true;

                case "emulator_bluestacks_fps":
                    string confPath = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.CommonApplicationData), @"BlueStacks_nxt\bluestacks.conf");
                    if (File.Exists(confPath))
                    {
                        try
                        {
                            string text = File.ReadAllText(confPath);
                            isApplied = text.Contains("enable_high_fps=\"1\"");
                        }
                        catch { isApplied = false; }
                    }
                    else
                    {
                        isApplied = false;
                    }
                    return true;

                case "cpu_core_parking":
                    string queryPark = RunToolSilent("powercfg.exe", "/query SCHEME_CURRENT SUB_PROCESSOR CPMINCORES");
                    isApplied = queryPark.IndexOf("0x00000064", StringComparison.OrdinalIgnoreCase) >= 0;
                    return true;

                case "delivery_optimization_p2p":
                    isApplied = GetDword(RegistryHive.LocalMachine, @"SOFTWARE\Policies\Microsoft\Windows\DeliveryOptimization", "DODownloadMode", -1) == 0;
                    return true;

                case "gpu_msi_mode":
                    isApplied = CheckGpuMsiMode();
                    return true;

                default:
                    return false;
            }
        }

        public static bool ApplyTweakNative(string id, Action<string> log)
        {
            switch (id)
            {
                case "win11_classic_context_menu":
                    SetString(RegistryHive.CurrentUser, @"Software\Classes\CLSID\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}\InprocServer32", "", "");
                    return true;

                case "win11_taskbar_clean":
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced", "TaskbarDa", 0);
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced", "TaskbarMn", 0);
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced", "ShowCopilotButton", 0);
                    return true;

                case "win11_explorer_clean":
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Explorer", "ShowRecommendations", 0);
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Explorer", "ShowRecent", 0);
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Explorer", "ShowFrequent", 0);
                    return true;

                case "win10_news_interests":
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Feeds", "ShellFeedsTaskbarViewMode", 2);
                    return true;

                case "win10_cortana_clean":
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced", "ShowCortanaButton", 0);
                    return true;

                case "win10_timeline_clean":
                    SetDword(RegistryHive.LocalMachine, @"SOFTWARE\Policies\Microsoft\Windows\System", "EnableActivityFeed", 0);
                    SetDword(RegistryHive.LocalMachine, @"SOFTWARE\Policies\Microsoft\Windows\System", "PublishUserActivities", 0);
                    SetDword(RegistryHive.LocalMachine, @"SOFTWARE\Policies\Microsoft\Windows\System", "UploadUserActivities", 0);
                    return true;

                case "bing_search_start":
                    SetDword(RegistryHive.CurrentUser, @"Software\Policies\Microsoft\Windows\Explorer", "DisableSearchBoxSuggestions", 1);
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Search", "BingSearchEnabled", 0);
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Search", "CortanaConsent", 0);
                    return true;

                case "consumer_features":
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager", "SystemPaneSuggestionsEnabled", 0);
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager", "SilentInstalledAppsEnabled", 0);
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager", "SoftLandingEnabled", 0);
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager", "SubscribedContent-338388Enabled", 0);
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager", "SubscribedContent-338389Enabled", 0);
                    return true;

                case "telemetry_services":
                    SetServiceStartMode("DiagTrack", 3);
                    SetServiceStartMode("dmwappushservice", 3);
                    StopServiceNative("DiagTrack");
                    StopServiceNative("dmwappushservice");
                    return true;

                case "telemetry_registry":
                    SetDword(RegistryHive.LocalMachine, @"SOFTWARE\Policies\Microsoft\Windows\DataCollection", "AllowTelemetry", 0);
                    return true;

                case "advertising_id":
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\AdvertisingInfo", "Enabled", 0);
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Privacy", "TailoredExperiencesWithDiagnosticDataEnabled", 0);
                    return true;

                case "game_dvr":
                    SetDword(RegistryHive.CurrentUser, @"SOFTWARE\Microsoft\Windows\CurrentVersion\GameDVR", "AppCaptureEnabled", 0);
                    SetDword(RegistryHive.CurrentUser, @"System\GameConfigStore", "GameDVR_Enabled", 0);
                    SetDword(RegistryHive.LocalMachine, @"SOFTWARE\Policies\Microsoft\Windows\GameDVR", "AllowGameDVR", 0);
                    return true;

                case "mouse_acceleration":
                    SetString(RegistryHive.CurrentUser, @"Control Panel\Mouse", "MouseSpeed", "0");
                    SetString(RegistryHive.CurrentUser, @"Control Panel\Mouse", "MouseThreshold1", "0");
                    SetString(RegistryHive.CurrentUser, @"Control Panel\Mouse", "MouseThreshold2", "0");
                    return true;

                case "ultimate_performance_plan":
                    string ultimateGuid = "e9a42b02-d5df-448d-aa00-03f14749eb61";
                    RunToolSilent("powercfg.exe", "-duplicatescheme " + ultimateGuid);
                    string setRes = RunToolSilent("powercfg.exe", "/setactive " + ultimateGuid);
                    if (setRes.IndexOf("error", StringComparison.OrdinalIgnoreCase) >= 0)
                    {
                        RunToolSilent("powercfg.exe", "/setactive 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c");
                    }
                    return true;

                case "hags_scheduling":
                    SetDword(RegistryHive.LocalMachine, @"SYSTEM\CurrentControlSet\Control\GraphicsDrivers", "HwSchMode", 2);
                    return true;

                case "network_autotuning":
                    RunToolSilent("netsh.exe", "int tcp set global autotuninglevel=normal");
                    RunToolSilent("netsh.exe", "int tcp set global ecncapability=disabled");
                    return true;

                case "fast_ntfs_access":
                    SetDword(RegistryHive.LocalMachine, @"SYSTEM\CurrentControlSet\Control\FileSystem", "NtfsDisableLastAccessUpdate", 1);
                    return true;

                case "emulator_bluestacks_fps":
                    ApplyBlueStacksFpsNative();
                    return true;

                case "cpu_core_parking":
                    RunToolSilent("powercfg.exe", "-setacvalueindex SCHEME_CURRENT 54533251-82be-4824-96c1-47b60b740d00 0cc5b647-c1df-4637-891a-dec35c318583 100");
                    RunToolSilent("powercfg.exe", "-setacvalueindex SCHEME_CURRENT 54533251-82be-4824-96c1-47b60b740d00 ea062031-0e34-4ff1-9b6d-eb1059334028 100");
                    RunToolSilent("powercfg.exe", "-setactive SCHEME_CURRENT");
                    return true;

                case "delivery_optimization_p2p":
                    SetDword(RegistryHive.LocalMachine, @"SOFTWARE\Policies\Microsoft\Windows\DeliveryOptimization", "DODownloadMode", 0);
                    SetServiceStartMode("DoSvc", 3);
                    return true;

                case "gpu_msi_mode":
                    SetGpuMsiMode(1);
                    return true;

                default:
                    return false;
            }
        }

        public static bool RevertTweakNative(string id, Action<string> log)
        {
            switch (id)
            {
                case "win11_classic_context_menu":
                    DeleteSubKeyTree(RegistryHive.CurrentUser, @"Software\Classes\CLSID\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}");
                    return true;

                case "win11_taskbar_clean":
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced", "TaskbarDa", 1);
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced", "TaskbarMn", 1);
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced", "ShowCopilotButton", 1);
                    return true;

                case "win11_explorer_clean":
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Explorer", "ShowRecommendations", 1);
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Explorer", "ShowRecent", 1);
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Explorer", "ShowFrequent", 1);
                    return true;

                case "win10_news_interests":
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Feeds", "ShellFeedsTaskbarViewMode", 0);
                    return true;

                case "win10_cortana_clean":
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced", "ShowCortanaButton", 1);
                    return true;

                case "win10_timeline_clean":
                    DeleteValue(RegistryHive.LocalMachine, @"SOFTWARE\Policies\Microsoft\Windows\System", "EnableActivityFeed");
                    DeleteValue(RegistryHive.LocalMachine, @"SOFTWARE\Policies\Microsoft\Windows\System", "PublishUserActivities");
                    return true;

                case "bing_search_start":
                    DeleteValue(RegistryHive.CurrentUser, @"Software\Policies\Microsoft\Windows\Explorer", "DisableSearchBoxSuggestions");
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Search", "BingSearchEnabled", 1);
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Search", "CortanaConsent", 1);
                    return true;

                case "consumer_features":
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager", "SystemPaneSuggestionsEnabled", 1);
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager", "SilentInstalledAppsEnabled", 1);
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager", "SoftLandingEnabled", 1);
                    return true;

                case "telemetry_services":
                    SetServiceStartMode("DiagTrack", 2);
                    SetServiceStartMode("dmwappushservice", 2);
                    StartServiceNative("DiagTrack");
                    return true;

                case "telemetry_registry":
                    DeleteValue(RegistryHive.LocalMachine, @"SOFTWARE\Policies\Microsoft\Windows\DataCollection", "AllowTelemetry");
                    return true;

                case "advertising_id":
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\AdvertisingInfo", "Enabled", 1);
                    SetDword(RegistryHive.CurrentUser, @"Software\Microsoft\Windows\CurrentVersion\Privacy", "TailoredExperiencesWithDiagnosticDataEnabled", 1);
                    return true;

                case "game_dvr":
                    SetDword(RegistryHive.CurrentUser, @"SOFTWARE\Microsoft\Windows\CurrentVersion\GameDVR", "AppCaptureEnabled", 1);
                    SetDword(RegistryHive.CurrentUser, @"System\GameConfigStore", "GameDVR_Enabled", 1);
                    DeleteValue(RegistryHive.LocalMachine, @"SOFTWARE\Policies\Microsoft\Windows\GameDVR", "AllowGameDVR");
                    return true;

                case "mouse_acceleration":
                    SetString(RegistryHive.CurrentUser, @"Control Panel\Mouse", "MouseSpeed", "1");
                    SetString(RegistryHive.CurrentUser, @"Control Panel\Mouse", "MouseThreshold1", "6");
                    SetString(RegistryHive.CurrentUser, @"Control Panel\Mouse", "MouseThreshold2", "10");
                    return true;

                case "ultimate_performance_plan":
                    RunToolSilent("powercfg.exe", "/setactive 381b4222-f694-41f0-9685-ff5bb260df2e");
                    return true;

                case "hags_scheduling":
                    SetDword(RegistryHive.LocalMachine, @"SYSTEM\CurrentControlSet\Control\GraphicsDrivers", "HwSchMode", 1);
                    return true;

                case "network_autotuning":
                    RunToolSilent("netsh.exe", "int tcp set global autotuninglevel=normal");
                    return true;

                case "fast_ntfs_access":
                    SetDword(RegistryHive.LocalMachine, @"SYSTEM\CurrentControlSet\Control\FileSystem", "NtfsDisableLastAccessUpdate", 2);
                    return true;

                case "emulator_bluestacks_fps":
                    RevertBlueStacksFpsNative();
                    return true;

                case "cpu_core_parking":
                    RunToolSilent("powercfg.exe", "-setacvalueindex SCHEME_CURRENT 54533251-82be-4824-96c1-47b60b740d00 0cc5b647-c1df-4637-891a-dec35c318583 5");
                    RunToolSilent("powercfg.exe", "-setacvalueindex SCHEME_CURRENT 54533251-82be-4824-96c1-47b60b740d00 ea062031-0e34-4ff1-9b6d-eb1059334028 100");
                    RunToolSilent("powercfg.exe", "-setactive SCHEME_CURRENT");
                    return true;

                case "delivery_optimization_p2p":
                    DeleteValue(RegistryHive.LocalMachine, @"SOFTWARE\Policies\Microsoft\Windows\DeliveryOptimization", "DODownloadMode");
                    SetServiceStartMode("DoSvc", 2);
                    StartServiceNative("DoSvc");
                    return true;

                case "gpu_msi_mode":
                    SetGpuMsiMode(0);
                    return true;

                default:
                    return false;
            }
        }
        #endregion

        #region Specialized Hardware Native Helpers
        private static bool CheckGpuMsiMode()
        {
            try
            {
                using (var baseKey = RegistryKey.OpenBaseKey(RegistryHive.LocalMachine, RegistryView.Registry64))
                using (var pciKey = baseKey.OpenSubKey(@"SYSTEM\CurrentControlSet\Enum\PCI", false))
                {
                    if (pciKey == null) return false;
                    foreach (string deviceKeyName in pciKey.GetSubKeyNames())
                    {
                        using (var devKey = pciKey.OpenSubKey(deviceKeyName, false))
                        {
                            if (devKey == null) continue;
                            foreach (string instName in devKey.GetSubKeyNames())
                            {
                                string targetPath = string.Format(@"SYSTEM\CurrentControlSet\Enum\PCI\{0}\{1}\Device Parameters\Interrupt Management\MessageSignaledInterruptProperties", deviceKeyName, instName);
                                int msi = GetDword(RegistryHive.LocalMachine, targetPath, "MSISupported", -1);
                                if (msi == 1) return true;
                            }
                        }
                    }
                }
            }
            catch { }
            return false;
        }

        private static void SetGpuMsiMode(int enabled)
        {
            try
            {
                using (var baseKey = RegistryKey.OpenBaseKey(RegistryHive.LocalMachine, RegistryView.Registry64))
                using (var pciKey = baseKey.OpenSubKey(@"SYSTEM\CurrentControlSet\Enum\PCI", false))
                {
                    if (pciKey == null) return;
                    foreach (string deviceKeyName in pciKey.GetSubKeyNames())
                    {
                        using (var devKey = pciKey.OpenSubKey(deviceKeyName, false))
                        {
                            if (devKey == null) continue;
                            foreach (string instName in devKey.GetSubKeyNames())
                            {
                                string devParamPath = string.Format(@"SYSTEM\CurrentControlSet\Enum\PCI\{0}\{1}\Device Parameters", deviceKeyName, instName);
                                using (var dpKey = baseKey.OpenSubKey(devParamPath, false))
                                {
                                    if (dpKey != null)
                                    {
                                        string targetPath = devParamPath + @"\Interrupt Management\MessageSignaledInterruptProperties";
                                        SetDword(RegistryHive.LocalMachine, targetPath, "MSISupported", enabled);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            catch { }
        }

        private static void ApplyBlueStacksFpsNative()
        {
            try
            {
                string confPath = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.CommonApplicationData), @"BlueStacks_nxt\bluestacks.conf");
                if (!File.Exists(confPath)) return;

                string backupPath = confPath + ".bak_original";
                if (!File.Exists(backupPath))
                {
                    File.Copy(confPath, backupPath, true);
                }

                string content = File.ReadAllText(confPath);
                string inst = "Pie64";
                var m = Regex.Match(content, @"bst\.installed_images=""([^""]+)""");
                if (m.Success) inst = m.Groups[1].Value;

                content = Regex.Replace(content, @"bst\.instance\." + inst + @"\.enable_high_fps=""[01]""", @"bst.instance." + inst + @".enable_high_fps=""1""");
                content = Regex.Replace(content, @"bst\.instance\." + inst + @"\.max_fps=""\d+""", @"bst.instance." + inst + @".max_fps=""240""");
                content = Regex.Replace(content, @"bst\.instance\." + inst + @"\.enable_fps_display=""[01]""", @"bst.instance." + inst + @".enable_fps_display=""1""");

                File.WriteAllText(confPath, content, new UTF8Encoding(false));
            }
            catch { }
        }

        private static void RevertBlueStacksFpsNative()
        {
            try
            {
                string confPath = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.CommonApplicationData), @"BlueStacks_nxt\bluestacks.conf");
                string backupPath = confPath + ".bak_original";
                if (File.Exists(backupPath))
                {
                    File.Copy(backupPath, confPath, true);
                }
            }
            catch { }
        }

        private static string RunToolSilent(string fileName, string args)
        {
            try
            {
                var psi = new ProcessStartInfo
                {
                    FileName = fileName,
                    Arguments = args,
                    CreateNoWindow = true,
                    UseShellExecute = false,
                    RedirectStandardOutput = true,
                    RedirectStandardError = true
                };

                using (var p = Process.Start(psi))
                {
                    string output = p.StandardOutput.ReadToEnd();
                    p.WaitForExit(3000);
                    return output ?? string.Empty;
                }
            }
            catch (Exception ex)
            {
                return "Error: " + ex.Message;
            }
        }
        #endregion
    }
}
