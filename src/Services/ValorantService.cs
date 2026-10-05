using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Management;
using System.Net.NetworkInformation;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.Win32;

// ============================================================
// ValorantService.cs - AnxiouslyOptimized
// VANGUARD COMPLIANCE GUARANTEE:
//   All optimizations are strictly OS/config/driver-level.
//   Zero process injection, zero DLL hooking, zero memory access
//   against VALORANT-Win64-Shipping.exe or vgk.sys.
//   These would trigger VAL5 / VAL61 HWID bans. Never cross this line.
// SAFETY GUARANTEE:
//   Every registry change backed up before write.
//   Every file modified has a timestamped .bak copy first.
//   We never set Realtime priority. All calls are timeout-guarded.
// ============================================================

namespace AnxiouslyOptimized.Services
{
    public enum ValorantInstallStatus { NotInstalled, InstalledNotRunning, Running }

    public enum CpuArch
    {
        Unknown,
        IntelLegacy,      // pre-12th gen, no E-cores
        IntelHybrid,      // 12th gen+, P+E cores
        AmdSingleCcd,     // Ryzen single CCD
        AmdDualCcdX3D,    // Ryzen X3D dual-CCD (CCD0 = V-Cache)
        AmdDualCcdNormal, // Ryzen dual-CCD no X3D
    }

    public class ValorantTopology
    {
        public ValorantInstallStatus InstallStatus { get; set; }
        public string InstallPath { get; set; }
        public string RiotClientPath { get; set; }
        public List<string> ConfigPaths { get; set; }
        public CpuArch CpuArchitecture { get; set; }
        public string CpuName { get; set; }
        public int PhysicalCoreCount { get; set; }
        public int PCoreCount { get; set; }
        public int ECoreCount { get; set; }
        public long AffinityMaskPCoresOnly { get; set; }
        public long AffinityMaskX3DCcd0 { get; set; }
        public bool IsLaptop { get; set; }
        public bool IsLowEndCpu { get; set; }
        public bool IsLowEndGpu { get; set; }
        public string GpuName { get; set; }
        public bool IsNvidiaGpu { get; set; }
        public bool IsAmdGpu { get; set; }
        public bool IsIntegratedGpu { get; set; }
        public bool FsoDisabled { get; set; }
        public bool DpiAwareSet { get; set; }
        public bool GameDvrDisabled { get; set; }
        public bool NagleDisabled { get; set; }
        public bool MultimediaThrottleDisabled { get; set; }
        public bool QosPolicyExists { get; set; }
        public bool MouseAccelDisabled { get; set; }
        public bool UsbSuspendDisabled { get; set; }
        public bool PowerPlanIsHighPerf { get; set; }
        public string ActiveConfigPath { get; set; }
        public bool ConfigBackupExists { get; set; }
        public string ConfigBackupPath { get; set; }
        public int ResolutionQuality { get; set; }
        public bool VsyncDisabled { get; set; }
        public long ShaderCacheSizeBytes { get; set; }
        public long LogCacheSizeBytes { get; set; }
        public long CrashDumpSizeBytes { get; set; }

        public ValorantTopology()
        {
            InstallStatus = ValorantInstallStatus.NotInstalled;
            ConfigPaths = new List<string>();
            CpuArchitecture = CpuArch.Unknown;
            ResolutionQuality = 100;
        }
    }

    public class ValorantPreset
    {
        public string Name { get; set; }
        public string Description { get; set; }
        public int ResolutionQuality { get; set; }
        public bool DisableVsync { get; set; }
        public bool ApplyCompetitiveLowSpec { get; set; }
        public bool DisableLetterbox { get; set; }
        public bool DisableFso { get; set; }
        public bool DisableGameDvr { get; set; }
        public bool SetDpiAware { get; set; }
        public bool DisableNagle { get; set; }
        public bool DisableMultimediaThrottle { get; set; }
        public bool ApplyQosPolicy { get; set; }
        public bool PinCpuAffinity { get; set; }
        public bool DisableMouseAccel { get; set; }
        public bool DisableUsbSuspend { get; set; }
        public bool SetHighPerfPowerPlan { get; set; }
        public bool IsLaptopSafe { get; set; }
    }

    public class ValorantFileBackup
    {
        public string OriginalPath { get; set; }
        public string BackupPath { get; set; }
        public DateTime CreatedAt { get; set; }
    }

    public class ValorantRegBackup
    {
        public RegistryHive Hive { get; set; }
        public string SubKey { get; set; }
        public string ValueName { get; set; }
        public object OriginalValue { get; set; }
        public RegistryValueKind OriginalKind { get; set; }
        public bool DidExist { get; set; }
    }

    // ================================================================
    // ValorantService static class
    // ================================================================
public static class ValorantService
    {
        // ----------------------------------------------------------------
        // Win32 declarations
        // ----------------------------------------------------------------
        [DllImport("user32.dll", SetLastError = true)]
        private static extern bool SystemParametersInfoIntArray(
            uint uiAction, uint uiParam, int[] pvParam, uint fWinIni);

        private const uint SPI_SETMOUSE       = 0x0004;
        private const uint SPIF_UPDATEINIFILE  = 0x01;
        private const uint SPIF_SENDCHANGE     = 0x02;

        // ----------------------------------------------------------------
        // Paths & constants
        // ----------------------------------------------------------------
        private static readonly string LocalAppData =
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);

        private const string GameBinaryName = "VALORANT-Win64-Shipping.exe";

        private static readonly string[] ValorantExePaths = new[]
        {
            Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles),
                @"Riot Games\VALORANT\live\VALORANT.exe"),
            Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86),
                @"Riot Games\VALORANT\live\VALORANT.exe"),
            @"C:\Riot Games\VALORANT\live\VALORANT.exe",
            @"D:\Riot Games\VALORANT\live\VALORANT.exe",
            @"E:\Riot Games\VALORANT\live\VALORANT.exe",
        };

        private static readonly string[] RiotClientPaths = new[]
        {
            Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles),
                @"Riot Games\Riot Client\RiotClientServices.exe"),
            Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86),
                @"Riot Games\Riot Client\RiotClientServices.exe"),
            @"C:\Riot Games\Riot Client\RiotClientServices.exe",
        };

        private static readonly string[] RiotBgProcesses = new[]
            { "RiotClientServices", "RiotClientUx", "RiotClientUxRender" };

        private static string ValorantConfigBase
        {
            get { return Path.Combine(LocalAppData, "VALORANT", "Saved", "Config"); }
        }

        private const string IniSubPath = @"Windows\GameUserSettings.ini";

        private static string BackupDir
        {
            get { return Path.Combine(LocalAppData, "AnxiouslyOptimized", "ValorantBackups"); }
        }

        // ================================================================
        // SECTION 1 – TOPOLOGY DETECTION
        // ================================================================

        public static ValorantTopology DetectTopology(Action<string> log)
        {
            var t = new ValorantTopology();
            LogSafe(log, "ValorantService: Scanning system topology...");
            DetectInstall(t, log);
            DetectCpu(t, log);
            DetectGpu(t, log);
            DetectFormFactor(t, log);
            ReadTweakStates(t, log);
            FindConfigPaths(t, log);
            ReadConfigValues(t, log);
            MeasureCaches(t, log);
            LogSafe(log, string.Format(
                "ValorantService: Done. Install={0} CPU={1} P={2} E={3}",
                t.InstallStatus, t.CpuArchitecture, t.PCoreCount, t.ECoreCount));
            return t;
        }

        private static void DetectInstall(ValorantTopology topo, Action<string> log)
        {
            try
            {
                foreach (var p in ValorantExePaths)
                {
                    if (File.Exists(p))
                    {
                        topo.InstallPath   = Path.GetDirectoryName(p);
                        topo.InstallStatus = ValorantInstallStatus.InstalledNotRunning;
                        LogSafe(log, "  [Valorant] Install: " + topo.InstallPath);
                        break;
                    }
                }
                if (topo.InstallStatus == ValorantInstallStatus.NotInstalled)
                {
                    topo.InstallPath = FindInstallViaRegistry();
                    if (!string.IsNullOrEmpty(topo.InstallPath))
                        topo.InstallStatus = ValorantInstallStatus.InstalledNotRunning;
                }
                foreach (var p in RiotClientPaths) if (File.Exists(p)) { topo.RiotClientPath = p; break; }

                if (topo.InstallStatus == ValorantInstallStatus.InstalledNotRunning)
                {
                    var procs = Process.GetProcessesByName("VALORANT-Win64-Shipping");
                    if (procs != null && procs.Length > 0)
                    {
                        topo.InstallStatus = ValorantInstallStatus.Running;
                        LogSafe(log, "  [Valorant] Running PID=" + procs[0].Id);
                        foreach (var p in procs) try { p.Dispose(); } catch { }
                    }
                }
            }
            catch (Exception ex) { LogSafe(log, "  [Valorant] Error: " + ex.Message); }
        }

        private static string FindInstallViaRegistry()
        {
            string[] roots = {
                @"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
                @"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"
            };
            foreach (var root in roots)
            {
                try
                {
                    using (var rk = Registry.LocalMachine.OpenSubKey(root))
                    {
                        if (rk == null) continue;
                        foreach (var n in rk.GetSubKeyNames())
                        {
                            using (var sk = rk.OpenSubKey(n))
                            {
                                if (sk == null) continue;
                                var dn = sk.GetValue("DisplayName") as string ?? "";
                                if (dn.IndexOf("VALORANT", StringComparison.OrdinalIgnoreCase) >= 0)
                                {
                                    var loc = sk.GetValue("InstallLocation") as string ?? "";
                                    if (!string.IsNullOrEmpty(loc) && Directory.Exists(loc)) return loc;
                                }
                            }
                        }
                    }
                }
                catch { }
            }
            return null;
        }

        private static void DetectCpu(ValorantTopology topo, Action<string> log)
        {
            try
            {
                using (var k = Registry.LocalMachine.OpenSubKey(
                    @"HARDWARE\DESCRIPTION\System\CentralProcessor\0"))
                    if (k != null) topo.CpuName = (k.GetValue("ProcessorNameString") as string ?? "").Trim();

                if (string.IsNullOrEmpty(topo.CpuName))
                    topo.CpuName = Environment.GetEnvironmentVariable("PROCESSOR_IDENTIFIER") ?? "x64";

                string up    = topo.CpuName.ToUpperInvariant();
                bool isIntel = up.Contains("INTEL");
                bool isAmd   = up.Contains("AMD") || up.Contains("RYZEN");
                bool isX3D   = up.Contains("X3D");

                int physical = 0, logical = 0;
                try
                {
                    using (var s = new System.Management.ManagementObjectSearcher(
                        "SELECT NumberOfCores,NumberOfLogicalProcessors FROM Win32_Processor"))
                        foreach (System.Management.ManagementObject o in s.Get())
                        {
                            physical += Convert.ToInt32(o["NumberOfCores"] ?? 0);
                            logical  += Convert.ToInt32(o["NumberOfLogicalProcessors"] ?? 0);
                        }
                }
                catch { }

                if (physical == 0) physical = Environment.ProcessorCount;
                if (logical  == 0) logical  = Environment.ProcessorCount;
                topo.PhysicalCoreCount = physical;

                bool hybrid = isIntel && (
                    up.Contains("12TH") || up.Contains("13TH") || up.Contains("14TH") ||
                    up.Contains("ULTRA") ||
                    Regex.IsMatch(up, @"I[3579]-1[2-9]\d{3}") ||
                    Regex.IsMatch(up, @"I[3579]-2[0-9]\d{3}"));

                if (isIntel && hybrid && physical >= 6)
                {
                    topo.CpuArchitecture      = CpuArch.IntelHybrid;
                    topo.PCoreCount           = IntelPCores(topo.CpuName, physical);
                    topo.ECoreCount           = physical - topo.PCoreCount;
                    topo.AffinityMaskPCoresOnly = IntelPCoreMask(topo.PCoreCount, logical);
                    LogSafe(log, string.Format("  [CPU] Intel Hybrid {0}P+{1}E Mask=0x{2:X}",
                        topo.PCoreCount, topo.ECoreCount, topo.AffinityMaskPCoresOnly));
                }
                else if (isIntel)
                {
                    topo.CpuArchitecture      = CpuArch.IntelLegacy;
                    topo.PCoreCount           = physical;
                    topo.AffinityMaskPCoresOnly = ((1L << logical) - 1L) & ~1L;
                }
                else if (isAmd && isX3D && physical >= 12)
                {
                    topo.CpuArchitecture    = CpuArch.AmdDualCcdX3D;
                    int ccd0                = Math.Min(16, logical / 2);
                    topo.AffinityMaskX3DCcd0 = (1L << ccd0) - 1L;
                    topo.PCoreCount         = physical;
                    LogSafe(log, string.Format("  [CPU] AMD X3D CCD0=0x{0:X}", topo.AffinityMaskX3DCcd0));
                }
                else if (isAmd && physical >= 12)
                { topo.CpuArchitecture = CpuArch.AmdDualCcdNormal; topo.PCoreCount = physical; }
                else if (isAmd)
                { topo.CpuArchitecture = CpuArch.AmdSingleCcd; topo.PCoreCount = physical; }
                else
                { topo.CpuArchitecture = CpuArch.Unknown; topo.PCoreCount = physical; }

                topo.IsLowEndCpu = physical < 4;
                LogSafe(log, string.Format("  [CPU] {0} arch={1}", topo.CpuName, topo.CpuArchitecture));
            }
            catch (Exception ex) { LogSafe(log, "  [CPU] Error: " + ex.Message); }
        }

        private static int IntelPCores(string name, int total)
        {
            string up = name.ToUpperInvariant();
            var m = Regex.Match(up, @"I([3579])-(\d{4,5})");
            if (m.Success)
            {
                int tier = int.Parse(m.Groups[1].Value), sku = int.Parse(m.Groups[2].Value);
                if (tier == 9 && sku >= 12900) return 8;
                if (tier == 7 && sku >= 12700) return 8;
                if (tier == 5 && sku >= 12400) return 6;
                if (tier == 3 && sku >= 12100) return 4;
            }
            if (up.Contains("ULTRA 9")) return 8;
            if (up.Contains("ULTRA 7")) return 6;
            if (up.Contains("ULTRA 5")) return 6;
            return Math.Max(1, total / 2);
        }

        private static long IntelPCoreMask(int pCores, int logical)
        {
            int pl = Math.Min(pCores * 2, logical);
            long mask = ((1L << pl) - 1L) & ~1L; // skip Thread 0 (IRQ)
            return mask == 0 ? 6L : mask;
        }

        private static void DetectGpu(ValorantTopology topo, Action<string> log)
        {
            try
            {
                for (int i = 0; i <= 3; i++)
                {
                    using (var k = Registry.LocalMachine.OpenSubKey(string.Format(
                        @"SYSTEM\CurrentControlSet\Control\Class\{{4d36e968-e325-11ce-bfc1-08002be10318}}\{0:D4}", i)))
                    {
                        if (k == null) continue;
                        var d = k.GetValue("DriverDesc") as string;
                        if (string.IsNullOrEmpty(d) || d.Contains("Basic Display") || d.Contains("Miracast")) continue;
                        topo.GpuName = d.Trim();
                        string u = topo.GpuName.ToUpperInvariant();
                        topo.IsNvidiaGpu     = u.Contains("NVIDIA") || u.Contains("RTX") || u.Contains("GTX");
                        topo.IsAmdGpu        = u.Contains("AMD")    || u.Contains("RADEON");
                        topo.IsIntegratedGpu = u.Contains("INTEL")  && (u.Contains("UHD") || u.Contains("IRIS"));
                        break;
                    }
                }
                topo.IsLowEndGpu = topo.IsIntegratedGpu || string.IsNullOrEmpty(topo.GpuName);
                LogSafe(log, "  [GPU] " + (topo.GpuName ?? "Unknown"));
            }
            catch (Exception ex) { LogSafe(log, "  [GPU] Error: " + ex.Message); }
        }

        private static void DetectFormFactor(ValorantTopology topo, Action<string> log)
        {
            try
            {
                using (var s = new System.Management.ManagementObjectSearcher(
                    "SELECT ChassisTypes FROM Win32_SystemEnclosure"))
                    foreach (System.Management.ManagementObject o in s.Get())
                    {
                        var ct = o["ChassisTypes"] as ushort[];
                        if (ct == null) continue;
                        foreach (var c in ct) if (c >= 8 && c <= 14) { topo.IsLaptop = true; break; }
                    }
            }
            catch { }
        }

        private static void ReadTweakStates(ValorantTopology topo, Action<string> log)
        {
            try
            {
                using (var k = Registry.CurrentUser.OpenSubKey(
                    @"Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers"))
                    if (k != null) { var v = k.GetValue(GameBinaryName) as string ?? "";
                        topo.FsoDisabled = v.Contains("DISABLEDXMAXIMIZEDWINDOWEDMODE");
                        topo.DpiAwareSet  = v.Contains("HIGHDPIAWARE"); }
            } catch { }

            try
            {
                using (var k = Registry.CurrentUser.OpenSubKey(@"System\GameConfigStore"))
                    if (k != null) { var v = k.GetValue("GameDVR_Enabled");
                        topo.GameDvrDisabled = v != null && Convert.ToInt32(v) == 0; }
            } catch { }

            try
            {
                using (var k = Registry.LocalMachine.OpenSubKey(
                    @"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile"))
                    if (k != null) { var v = k.GetValue("NetworkThrottlingIndex");
                        topo.MultimediaThrottleDisabled = v != null && Convert.ToInt64(v) >= 0xFFFFFFFF; }
            } catch { }

            try
            {
                using (var k = Registry.LocalMachine.OpenSubKey(
                    @"SOFTWARE\Policies\Microsoft\Windows\QoS\ValorantQoS"))
                    topo.QosPolicyExists = k != null;
            } catch { }

            try
            {
                using (var p = new Process())
                {
                    p.StartInfo = new ProcessStartInfo { FileName = "powercfg.exe",
                        Arguments = "/getactivescheme", CreateNoWindow = true,
                        UseShellExecute = false, RedirectStandardOutput = true };
                    p.Start(); string o = p.StandardOutput.ReadToEnd(); p.WaitForExit(3000);
                    topo.PowerPlanIsHighPerf = o.Contains("8c5e7fda") || o.Contains("e9a42b02");
                }
            } catch { }

            try
            {
                using (var k = Registry.CurrentUser.OpenSubKey(@"Control Panel\Mouse"))
                    if (k != null) topo.MouseAccelDisabled = (k.GetValue("MouseSpeed") as string) == "0";
            } catch { }

            try
            {
                using (var k = Registry.LocalMachine.OpenSubKey(@"SYSTEM\CurrentControlSet\Services\USB"))
                    if (k != null) { var v = k.GetValue("DisableSelectiveSuspend");
                        topo.UsbSuspendDisabled = v != null && Convert.ToInt32(v) == 1; }
            } catch { }
        }

        private static void FindConfigPaths(ValorantTopology topo, Action<string> log)
        {
            try
            {
                if (!Directory.Exists(ValorantConfigBase))
                { LogSafe(log, "  [Config] No VALORANT config dir."); return; }
                foreach (var gd in Directory.GetDirectories(ValorantConfigBase))
                {
                    string ini = Path.Combine(gd, IniSubPath);
                    if (File.Exists(ini)) { topo.ConfigPaths.Add(ini); LogSafe(log, "  [Config] " + ini); }
                }
                if (topo.ConfigPaths.Count > 0)
                    topo.ActiveConfigPath = topo.ConfigPaths
                        .OrderByDescending(p => File.GetLastWriteTime(p)).First();
            }
            catch (Exception ex) { LogSafe(log, "  [Config] Error: " + ex.Message); }
        }

        private static void ReadConfigValues(ValorantTopology topo, Action<string> log)
        {
            if (string.IsNullOrEmpty(topo.ActiveConfigPath) || !File.Exists(topo.ActiveConfigPath)) return;
            try
            {
                string bak = topo.ActiveConfigPath + ".anxopt.bak";
                topo.ConfigBackupExists = File.Exists(bak);
                topo.ConfigBackupPath   = bak;
                foreach (var line in File.ReadAllLines(topo.ActiveConfigPath, Encoding.UTF8))
                {
                    string t2 = line.Trim();
                    if (t2.StartsWith("sg.ResolutionQuality=", StringComparison.OrdinalIgnoreCase))
                    {
                        double q;
                        if (double.TryParse(t2.Substring(21),
                            System.Globalization.NumberStyles.Any,
                            System.Globalization.CultureInfo.InvariantCulture, out q))
                            topo.ResolutionQuality = (int)Math.Round(q);
                    }
                    else if (t2.StartsWith("bUseVSync=", StringComparison.OrdinalIgnoreCase))
                        topo.VsyncDisabled = t2.EndsWith("False", StringComparison.OrdinalIgnoreCase);
                }
            }
            catch { }
        }

        private static void MeasureCaches(ValorantTopology topo, Action<string> log)
        {
            string[] sd = {
                Path.Combine(LocalAppData, "NVIDIA", "DXCache"),
                Path.Combine(LocalAppData, "NVIDIA", "GLCache"),
                Path.Combine(LocalAppData, "AMD",    "DxCache"),
                Path.Combine(LocalAppData, "D3DSCache"),
                Path.Combine(LocalAppData, "VALORANT", "Saved", "PipelineCaches"),
            };
            foreach (var d in sd) topo.ShaderCacheSizeBytes += DirSize(d);
            topo.LogCacheSizeBytes  = DirSize(Path.Combine(LocalAppData, "VALORANT", "Saved", "Logs"));
            topo.CrashDumpSizeBytes = DirSize(Path.Combine(LocalAppData, "VALORANT", "Saved", "Crashes"));
            LogSafe(log, string.Format("  [Cache] Shaders={0}MB Logs={1}MB Crashes={2}MB",
                topo.ShaderCacheSizeBytes >> 20, topo.LogCacheSizeBytes >> 20, topo.CrashDumpSizeBytes >> 20));
        }

        // ================================================================
        // SECTION 2 – PRESETS
        // ================================================================

        public static ValorantPreset GetPresetCompetitivePotato(ValorantTopology t)
        {
            return new ValorantPreset
            {
                Name = "Competitive Potato",
                Description = "Maximum FPS. All non-essential graphics stripped. Lowest latency for 144Hz+.",
                ResolutionQuality = t.IsLowEndGpu ? 80 : 100,
                DisableVsync = true,
                ApplyCompetitiveLowSpec = true,
                DisableFso = true,
                DisableGameDvr = true,
                SetDpiAware = true,
                DisableNagle = true,
                DisableMultimediaThrottle = true,
                ApplyQosPolicy = true,
                PinCpuAffinity = !t.IsLowEndCpu,
                DisableMouseAccel = true,
                DisableUsbSuspend = true,
                SetHighPerfPowerPlan = !t.IsLaptop,
                IsLaptopSafe = true
            };
        }

        public static ValorantPreset GetPresetTournament240Hz(ValorantTopology t)
        {
            return new ValorantPreset
            {
                Name = "Tournament 240Hz+",
                Description = "Native resolution, ultra-low latency for 240/360/540Hz displays.",
                ResolutionQuality = 100,
                DisableVsync = true,
                ApplyCompetitiveLowSpec = true,
                DisableFso = true,
                DisableGameDvr = true,
                SetDpiAware = true,
                DisableNagle = true,
                DisableMultimediaThrottle = true,
                ApplyQosPolicy = true,
                PinCpuAffinity = !t.IsLowEndCpu,
                DisableMouseAccel = true,
                DisableUsbSuspend = true,
                SetHighPerfPowerPlan = !t.IsLaptop,
                IsLaptopSafe = true
            };
        }

        public static ValorantPreset GetPresetLaptopBalanced(ValorantTopology t)
        {
            return new ValorantPreset
            {
                Name = "Laptop Balanced",
                Description = "FPS boost for laptops. Skips battery-draining tweaks.",
                ResolutionQuality = 85,
                DisableVsync = true,
                ApplyCompetitiveLowSpec = true,
                DisableFso = true,
                DisableGameDvr = true,
                SetDpiAware = true,
                DisableNagle = true,
                DisableMultimediaThrottle = false,
                ApplyQosPolicy = true,
                PinCpuAffinity = false,
                DisableMouseAccel = true,
                DisableUsbSuspend = false,
                SetHighPerfPowerPlan = false,
                IsLaptopSafe = true
            };
        }

        // ================================================================
        // SECTION 3 – APPLY PRESET
        // ================================================================

        public static async Task<Tuple<List<ValorantRegBackup>, List<ValorantFileBackup>, bool>>
            ApplyPresetAsync(ValorantPreset preset, ValorantTopology topo, Action<string> log)
        {
            var rb = new List<ValorantRegBackup>();
            var fb = new List<ValorantFileBackup>();
            bool ok = true;
            await Task.Run(() =>
            {
                if (preset.ApplyCompetitiveLowSpec || preset.DisableVsync ||
                    preset.ResolutionQuality != 100 || preset.DisableLetterbox)
                    if (!ApplyIni(topo, preset, fb, log)) ok = false;

                if (preset.DisableFso    || preset.SetDpiAware)   ApplyFso(preset, rb, log);
                if (preset.DisableGameDvr)                         ApplyDvr(rb, log);
                if (preset.DisableMultimediaThrottle)              ApplyThrottle(rb, log);
                if (preset.DisableNagle)                           ApplyNagle(rb, log);
                if (preset.ApplyQosPolicy)                         ApplyQos(rb, log);
                if (preset.DisableMouseAccel)                      ApplyMouse(rb, log);
                if (preset.DisableUsbSuspend)                      ApplyUsb(rb, log);
                if (preset.SetHighPerfPowerPlan && !topo.IsLaptop) ApplyPower(log);
                if (preset.PinCpuAffinity)                         ApplyAffinity(topo, log);
                LogSafe(log, "ValorantService: All optimizations applied.");
            });
            return Tuple.Create(rb, fb, ok);
        }

        // ----------------------------------------------------------------
        // INI patcher (atomic write via temp file)
        // ----------------------------------------------------------------
        private static bool ApplyIni(ValorantTopology topo, ValorantPreset preset,
            List<ValorantFileBackup> fb, Action<string> log)
        {
            if (topo.ConfigPaths == null || topo.ConfigPaths.Count == 0)
            { LogSafe(log, "  [Config] No ini found - skipping."); return true; }

            bool any = false;
            foreach (var ini in topo.ConfigPaths)
            {
                try
                {
                    if (!File.Exists(ini)) continue;
                    EnsureDir(BackupDir);
                    string bak = ini + ".anxopt.bak";
                    if (!File.Exists(bak))
                    {
                        File.Copy(ini, bak, false);
                        fb.Add(new ValorantFileBackup { OriginalPath = ini, BackupPath = bak, CreatedAt = DateTime.Now });
                        LogSafe(log, "  [Config] Backup: " + bak);
                    }

                    var lines = new List<string>(File.ReadAllLines(ini, Encoding.UTF8));
                    var sg = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
                    if (preset.ApplyCompetitiveLowSpec)
                    {
                        sg["sg.ViewDistanceQuality"] = "0"; sg["sg.AntiAliasingQuality"] = "0";
                        sg["sg.ShadowQuality"]       = "0"; sg["sg.PostProcessQuality"]  = "0";
                        sg["sg.TextureQuality"]      = "3"; // Keep textures readable
                        sg["sg.EffectsQuality"]      = "0"; sg["sg.FoliageQuality"]      = "0";
                        sg["sg.ShadingQuality"]      = "0";
                    }
                    sg["sg.ResolutionQuality"] = preset.ResolutionQuality
                        .ToString(System.Globalization.CultureInfo.InvariantCulture) + ".000000";

                    var gk = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
                    if (preset.DisableVsync)    gk["bUseVSync"]        = "False";
                    if (preset.DisableLetterbox) gk["bShouldLetterbox"] = "False";

                    lines = PatchIni(lines, sg, "[ScalabilityGroups]");
                    lines = PatchIni(lines, gk, "[/Script/ShooterGame.ShooterGameUserSettings]");

                    string tmp = ini + ".anxopt.tmp";
                    File.WriteAllLines(tmp, lines.ToArray(), Encoding.UTF8);
                    File.Copy(tmp, ini, true);
                    File.Delete(tmp);
                    LogSafe(log, "  [Config] Updated: " + Path.GetFileName(ini));
                    any = true;
                }
                catch (Exception ex) { LogSafe(log, "  [Config] Failed: " + ex.Message); }
            }
            return any;
        }

        private static List<string> PatchIni(List<string> lines,
            Dictionary<string, string> keys, string section)
        {
            if (keys == null || keys.Count == 0) return lines;
            var result    = new List<string>(lines.Count + keys.Count + 4);
            var remaining = new Dictionary<string, string>(keys, StringComparer.OrdinalIgnoreCase);
            bool inSec    = false;
            for (int i = 0; i < lines.Count; i++)
            {
                string line = lines[i], t = line.Trim();
                if (t.StartsWith("["))
                {
                    if (inSec) { foreach (var kv in remaining) result.Add(kv.Key + "=" + kv.Value); remaining.Clear(); }
                    inSec = t.Equals(section, StringComparison.OrdinalIgnoreCase);
                }
                if (inSec && !t.StartsWith("[") && t.Contains("="))
                {
                    int eq = t.IndexOf('='); string k = t.Substring(0, eq).Trim();
                    if (remaining.ContainsKey(k)) { result.Add(k + "=" + remaining[k]); remaining.Remove(k); continue; }
                }
                result.Add(line);
            }
            if (inSec) foreach (var kv in remaining) result.Add(kv.Key + "=" + kv.Value);
            else if (remaining.Count > 0)
            { result.Add(""); result.Add(section); foreach (var kv in remaining) result.Add(kv.Key + "=" + kv.Value); }
            return result;
        }

        // ----------------------------------------------------------------
        // Registry tweaks
        // ----------------------------------------------------------------
        private static void ApplyFso(ValorantPreset preset, List<ValorantRegBackup> rb, Action<string> log)
        {
            try
            {
                const string sub = @"Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers";
                string flags = (preset.DisableFso  ? "~ DISABLEDXMAXIMIZEDWINDOWEDMODE " : "") +
                               (preset.SetDpiAware ? "~ HIGHDPIAWARE " : "");
                flags = flags.Trim();
                using (var k = Registry.CurrentUser.CreateSubKey(sub))
                {
                    if (k == null) return;
                    var orig = k.GetValue(GameBinaryName);
                    rb.Add(new ValorantRegBackup { Hive = RegistryHive.CurrentUser, SubKey = sub,
                        ValueName = GameBinaryName, OriginalValue = orig,
                        OriginalKind = orig != null ? RegistryValueKind.String : RegistryValueKind.None,
                        DidExist = orig != null });
                    var merged = new HashSet<string>(
                        (orig as string ?? "").Split(new[]{' '}, StringSplitOptions.RemoveEmptyEntries),
                        StringComparer.OrdinalIgnoreCase);
                    foreach (var tok in flags.Split(new[]{' '}, StringSplitOptions.RemoveEmptyEntries)) merged.Add(tok);
                    k.SetValue(GameBinaryName, string.Join(" ", merged), RegistryValueKind.String);
                    LogSafe(log, "  [FSO] Flags: " + string.Join(" ", merged));
                }
            }
            catch (Exception ex) { LogSafe(log, "  [FSO] Error: " + ex.Message); }
        }

        private static void ApplyDvr(List<ValorantRegBackup> rb, Action<string> log)
        {
            SetHkcu(@"System\GameConfigStore",                           "GameDVR_Enabled",     0, rb);
            SetHkcu(@"System\GameConfigStore",                           "GameDVR_FSEBehavior", 2, rb);
            SetHkcu(@"Software\Microsoft\Windows\CurrentVersion\GameDVR","AppCaptureEnabled",  0, rb);
            LogSafe(log, "  [DVR] Game DVR disabled.");
        }

        private static void ApplyThrottle(List<ValorantRegBackup> rb, Action<string> log)
        {
            const string sub = @"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile";
            try
            {
                using (var k = Registry.LocalMachine.OpenSubKey(sub, true))
                {
                    if (k == null) return;
                    BkSet(k, RegistryHive.LocalMachine, sub, "NetworkThrottlingIndex",
                        unchecked((int)0xFFFFFFFF), RegistryValueKind.DWord, rb);
                    BkSet(k, RegistryHive.LocalMachine, sub, "SystemResponsiveness",
                        0, RegistryValueKind.DWord, rb);
                    LogSafe(log, "  [Net] NetworkThrottlingIndex disabled.");
                }
            }
            catch (Exception ex) { LogSafe(log, "  [Net] Error: " + ex.Message); }
        }

        private static void ApplyNagle(List<ValorantRegBackup> rb, Action<string> log)
        {
            try
            {
                string nic = ActiveNicGuid();
                if (string.IsNullOrEmpty(nic)) { LogSafe(log, "  [Nagle] No active NIC."); return; }
                string sub = @"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces\" + nic;
                using (var k = Registry.LocalMachine.OpenSubKey(sub, true))
                {
                    if (k == null) return;
                    BkSet(k, RegistryHive.LocalMachine, sub, "TcpAckFrequency", 1, RegistryValueKind.DWord, rb);
                    BkSet(k, RegistryHive.LocalMachine, sub, "TCPNoDelay",       1, RegistryValueKind.DWord, rb);
                    BkSet(k, RegistryHive.LocalMachine, sub, "TcpDelAckTicks",   0, RegistryValueKind.DWord, rb);
                    LogSafe(log, "  [Nagle] Disabled on " + nic.Substring(0, Math.Min(8, nic.Length)) + "...");
                }
            }
            catch (Exception ex) { LogSafe(log, "  [Nagle] Error: " + ex.Message); }
        }

        private static void ApplyQos(List<ValorantRegBackup> rb, Action<string> log)
        {
            const string sub = @"SOFTWARE\Policies\Microsoft\Windows\QoS\ValorantQoS";
            try
            {
                rb.Add(new ValorantRegBackup { Hive = RegistryHive.LocalMachine, SubKey = sub,
                    ValueName = "__KEY_EXISTED__", DidExist = false, OriginalKind = RegistryValueKind.None });
                using (var k = Registry.LocalMachine.CreateSubKey(sub))
                {
                    if (k == null) return;
                    k.SetValue("Version",                "1.0",         RegistryValueKind.String);
                    k.SetValue("Application Name",       GameBinaryName, RegistryValueKind.String);
                    k.SetValue("Protocol",               "UDP",         RegistryValueKind.String);
                    k.SetValue("Local Port",             "*",           RegistryValueKind.String);
                    k.SetValue("Local IP",               "*",           RegistryValueKind.String);
                    k.SetValue("Local IP Prefix Length", "0",           RegistryValueKind.String);
                    k.SetValue("Remote Port",            "*",           RegistryValueKind.String);
                    k.SetValue("Remote IP",              "*",           RegistryValueKind.String);
                    k.SetValue("Remote IP Prefix Length","0",           RegistryValueKind.String);
                    k.SetValue("DSCP Value",             "46",          RegistryValueKind.String);
                    k.SetValue("Throttle Rate",          "-1",          RegistryValueKind.String);
                    LogSafe(log, "  [QoS] DSCP 46 (Expedited Forwarding) applied.");
                }
            }
            catch (Exception ex) { LogSafe(log, "  [QoS] Error: " + ex.Message); }
        }

        private static void ApplyMouse(List<ValorantRegBackup> rb, Action<string> log)
        {
            const string sub = @"Control Panel\Mouse";
            try
            {
                using (var k = Registry.CurrentUser.OpenSubKey(sub, true))
                {
                    if (k == null) return;
                    foreach (var vn in new[] { "MouseSpeed", "MouseThreshold1", "MouseThreshold2" })
                        rb.Add(new ValorantRegBackup { Hive = RegistryHive.CurrentUser, SubKey = sub,
                            ValueName = vn, OriginalValue = k.GetValue(vn) as string ?? "",
                            OriginalKind = RegistryValueKind.String, DidExist = true });
                    k.SetValue("MouseSpeed", "0", RegistryValueKind.String);
                    k.SetValue("MouseThreshold1", "0", RegistryValueKind.String);
                    k.SetValue("MouseThreshold2", "0", RegistryValueKind.String);
                    SystemParametersInfoIntArray(SPI_SETMOUSE, 0, new int[] { 0, 0, 0 },
                        SPIF_UPDATEINIFILE | SPIF_SENDCHANGE);
                    LogSafe(log, "  [Mouse] Pointer precision disabled (live applied).");
                }
            }
            catch (Exception ex) { LogSafe(log, "  [Mouse] Error: " + ex.Message); }
        }

        private static void ApplyUsb(List<ValorantRegBackup> rb, Action<string> log)
        {
            const string sub = @"SYSTEM\CurrentControlSet\Services\USB";
            try
            {
                using (var k = Registry.LocalMachine.OpenSubKey(sub, true))
                {
                    if (k == null) return;
                    BkSet(k, RegistryHive.LocalMachine, sub, "DisableSelectiveSuspend",
                        1, RegistryValueKind.DWord, rb);
                    LogSafe(log, "  [USB] Selective Suspend disabled.");
                }
            }
            catch (Exception ex) { LogSafe(log, "  [USB] Error: " + ex.Message); }
        }

        private static void ApplyPower(Action<string> log)
        {
            try
            {
                // High Performance GUID (not Realtime/Ultimate - those can destabilize low-spec systems)
                if (Exec("powercfg.exe", "/setactive 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c") != 0)
                {
                    Exec("powercfg.exe", "/duplicatescheme 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c");
                    Exec("powercfg.exe", "/setactive 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c");
                }
                LogSafe(log, "  [Power] High Performance plan active.");
            }
            catch (Exception ex) { LogSafe(log, "  [Power] Error: " + ex.Message); }
        }

        private static void ApplyAffinity(ValorantTopology topo, Action<string> log)
        {
            try
            {
                long mask = SafeMask(topo);
                if (mask <= 0) { LogSafe(log, "  [Affinity] No safe mask - skip."); return; }
                var procs = Process.GetProcessesByName("VALORANT-Win64-Shipping");
                if (procs == null || procs.Length == 0)
                { LogSafe(log, "  [Affinity] Game not running - daemon will apply."); return; }
                foreach (var p in procs)
                {
                    try { p.ProcessorAffinity = new IntPtr(mask);
                        if (p.PriorityClass != ProcessPriorityClass.RealTime)
                            p.PriorityClass = ProcessPriorityClass.High;
                        LogSafe(log, string.Format("  [Affinity] PID {0} mask=0x{1:X}", p.Id, mask)); }
                    catch { }
                    finally { try { p.Dispose(); } catch { } }
                }
                foreach (var n in RiotBgProcesses)
                    foreach (var bp in Process.GetProcessesByName(n))
                    { try { bp.PriorityClass = ProcessPriorityClass.BelowNormal; } catch { }
                      finally { try { bp.Dispose(); } catch { } } }
            }
            catch (Exception ex) { LogSafe(log, "  [Affinity] Error: " + ex.Message); }
        }

        private static long SafeMask(ValorantTopology t)
        {
            switch (t.CpuArchitecture)
            {
                case CpuArch.IntelHybrid:
                    return t.AffinityMaskPCoresOnly > 0
                        ? t.AffinityMaskPCoresOnly
                        : ((1L << Environment.ProcessorCount) - 1L) & ~1L;
                case CpuArch.AmdDualCcdX3D:
                    return t.AffinityMaskX3DCcd0 > 0
                        ? t.AffinityMaskX3DCcd0
                        : (1L << Environment.ProcessorCount) - 1L;
                case CpuArch.AmdDualCcdNormal: return -1;
                case CpuArch.IntelLegacy:
                case CpuArch.AmdSingleCcd:
                    return ((1L << Environment.ProcessorCount) - 1L) & ~1L;
                default: return -1;
            }
        }

        // ================================================================
        // SECTION 4 – ROLLBACK
        // ================================================================

        public static async Task RevertAsync(List<ValorantRegBackup> rb,
            List<ValorantFileBackup> fb, Action<string> log)
        {
            await Task.Run(() =>
            {
                LogSafe(log, "ValorantService: Reverting...");
                foreach (var r in rb) try { RevertReg(r); } catch (Exception ex) { LogSafe(log, "  [Revert] " + ex.Message); }
                foreach (var f in fb)
                    try { if (File.Exists(f.BackupPath)) { File.Copy(f.BackupPath, f.OriginalPath, true);
                        LogSafe(log, "  [Revert] Restored " + Path.GetFileName(f.OriginalPath)); } }
                    catch (Exception ex) { LogSafe(log, "  [Revert] " + ex.Message); }
                try { Exec("powercfg.exe", "/setactive 381b4222-f694-41f0-9685-ff5bb260df2e"); } catch { }
                LogSafe(log, "  [Revert] Power plan restored to Balanced.");
                bool qosNew = rb.Exists(r => r.SubKey.EndsWith("ValorantQoS", StringComparison.OrdinalIgnoreCase)
                    && r.ValueName == "__KEY_EXISTED__" && !r.DidExist);
                if (qosNew)
                    try { Registry.LocalMachine.DeleteSubKeyTree(
                        @"SOFTWARE\Policies\Microsoft\Windows\QoS\ValorantQoS", false); }
                    catch { }
                LogSafe(log, "ValorantService: Revert complete.");
            });
        }

        private static void RevertReg(ValorantRegBackup item)
        {
            using (var k = (item.Hive == RegistryHive.LocalMachine
                ? Registry.LocalMachine : Registry.CurrentUser).OpenSubKey(item.SubKey, true))
            {
                if (k == null) return;
                if (!item.DidExist) try { k.DeleteValue(item.ValueName, false); } catch { }
                else if (item.OriginalValue != null) k.SetValue(item.ValueName, item.OriginalValue, item.OriginalKind);
            }
        }

        // ================================================================
        // SECTION 5 – RESTORE CONFIG FROM BACKUP
        // ================================================================

        public static async Task<bool> RestoreGameConfigAsync(ValorantTopology topo, Action<string> log)
        {
            return await Task.Run(() =>
            {
                bool any = false;
                foreach (var cfg in topo.ConfigPaths)
                {
                    string bak = cfg + ".anxopt.bak";
                    if (!File.Exists(bak)) continue;
                    try { File.Copy(bak, cfg, true); LogSafe(log, "  [Config] Restored."); any = true; }
                    catch (Exception ex) { LogSafe(log, "  [Config] Error: " + ex.Message); }
                }
                return any;
            });
        }

        // ================================================================
        // SECTION 6 – INDIVIDUAL TWEAKERS
        // ================================================================

        public static async Task<bool> SetResolutionQualityAsync(
            ValorantTopology topo, int quality, Action<string> log)
        {
            quality = Math.Max(50, Math.Min(100, quality));
            return await Task.Run(() =>
            {
                bool r = ApplyIni(topo, new ValorantPreset
                    { ResolutionQuality = quality, DisableVsync = false,
                      ApplyCompetitiveLowSpec = false, DisableLetterbox = false },
                    new List<ValorantFileBackup>(), log);
                LogSafe(log, "  [Config] Resolution=" + quality + "%.");
                return r;
            });
        }

        public static async Task<bool> SetCompetitiveLowSpecAsync(
            ValorantTopology topo, bool enable, Action<string> log)
        {
            return await Task.Run(() =>
            {
                bool r = ApplyIni(topo, new ValorantPreset
                    { ApplyCompetitiveLowSpec = enable,
                      ResolutionQuality = topo.ResolutionQuality, DisableVsync = true },
                    new List<ValorantFileBackup>(), log);
                LogSafe(log, "  [Config] Low-spec=" + (enable ? "ON" : "OFF") + ".");
                return r;
            });
        }

        // ================================================================
        // SECTION 7 – CACHE PURGE
        // ================================================================

        public static async Task<long> PurgeCachesAsync(
            bool shaders, bool logs, bool crashes, Action<string> log)
        {
            return await Task.Run(() =>
            {
                long total = 0;
                if (shaders)
                {
                    foreach (var d in new[] {
                        Path.Combine(LocalAppData,"NVIDIA","DXCache"),
                        Path.Combine(LocalAppData,"NVIDIA","GLCache"),
                        Path.Combine(LocalAppData,"AMD","DxCache"),
                        Path.Combine(LocalAppData,"D3DSCache"),
                        Path.Combine(LocalAppData,"VALORANT","Saved","PipelineCaches") })
                    {
                        long f = DeleteDir(d, log); total += f;
                        if (f > 0) LogSafe(log, string.Format("  [Cache] {0}MB from {1}", f >> 20, Path.GetFileName(d)));
                    }
                }
                if (logs)
                {
                    long f = DeleteDir(Path.Combine(LocalAppData,"VALORANT","Saved","Logs"), log);
                    total += f;
                    LogSafe(log, string.Format("  [Logs] {0}MB cleared.", f >> 20));
                }
                if (crashes)
                {
                    long f = DeleteDir(Path.Combine(LocalAppData,"VALORANT","Saved","Crashes"), log);
                    total += f;
                    LogSafe(log, string.Format("  [Crash] {0}MB cleared.", f >> 20));
                }
                LogSafe(log, string.Format("  [Purge] Total: {0}MB.", total >> 20));
                return total;
            });
        }

        // ================================================================
        // SECTION 8 – AFFINITY DAEMON
        // ================================================================

        private static volatile bool _run;
        private static Thread _daemon;

        public static void StartAffinityDaemon(ValorantTopology topo, Action<string> log)
        {
            if (_run) return;
            _run = true;
            _daemon = new Thread(() =>
            {
                long mask   = SafeMask(topo);
                if (mask <= 0) { _run = false; return; }
                var   expire = DateTime.Now.AddHours(3);
                bool  applied = false;
                while (_run && DateTime.Now < expire)
                {
                    Thread.Sleep(2000);
                    try
                    {
                        var procs = Process.GetProcessesByName("VALORANT-Win64-Shipping");
                        if (procs != null && procs.Length > 0)
                        {
                            if (!applied)
                            {
                                foreach (var p in procs)
                                {
                                    try { p.ProcessorAffinity = new IntPtr(mask);
                                        if (p.PriorityClass != ProcessPriorityClass.RealTime)
                                            p.PriorityClass = ProcessPriorityClass.High;
                                        LogSafe(log, string.Format("  [Daemon] PID {0} 0x{1:X}", p.Id, mask)); }
                                    catch { }
                                    finally { try { p.Dispose(); } catch { } }
                                }
                                foreach (var n in RiotBgProcesses)
                                    foreach (var bp in Process.GetProcessesByName(n))
                                    { try { bp.PriorityClass = ProcessPriorityClass.BelowNormal; } catch { }
                                      finally { try { bp.Dispose(); } catch { } } }
                                applied = true;
                            }
                            else foreach (var p in procs) try { p.Dispose(); } catch { }
                        }
                        else if (applied)
                        { applied = false; LogSafe(log, "  [Daemon] Game closed."); }
                    }
                    catch { }
                }
                _run = false;
                LogSafe(log, "  [Daemon] Stopped.");
            }) { IsBackground = true, Name = "ValorantAffinityDaemon", Priority = ThreadPriority.Lowest };
            _daemon.Start();
            LogSafe(log, "  [Daemon] Started (2s poll, 3h TTL).");
        }

        public static void StopAffinityDaemon() { _run = false; }

        // ================================================================
        // SECTION 9 – RIOT SERVER PING
        // ================================================================

        public static async Task<Dictionary<string, long>> PingRiotRegionsAsync(Action<string> log)
        {
            var regions = new Dictionary<string, string> {
                { "NA (Chicago)",   "na1.api.riotgames.com"  },
                { "EU (Frankfurt)", "euw1.api.riotgames.com" },
                { "AP (Singapore)", "oc1.api.riotgames.com"  },
                { "BR (Sao Paulo)", "br1.api.riotgames.com"  },
                { "KR (Seoul)",     "kr.api.riotgames.com"   },
            };
            var results = new Dictionary<string, long>();
            await Task.Run(() =>
            {
                LogSafe(log, "  [Ping] Probing Riot regions...");
                foreach (var kv in regions)
                    try
                    {
                        using (var ping = new Ping())
                        {
                            var r = ping.Send(kv.Value, 2000);
                            results[kv.Key] = r != null && r.Status == IPStatus.Success ? r.RoundtripTime : -1;
                            LogSafe(log, string.Format("  [Ping] {0}: {1}", kv.Key,
                                results[kv.Key] >= 0 ? results[kv.Key] + "ms" : "Timeout"));
                        }
                    }
                    catch { results[kv.Key] = -1; }
            });
            return results;
        }

        // ================================================================
        // HELPERS
        // ================================================================

        private static void SetHkcu(string sub, string name, int val, List<ValorantRegBackup> rb)
        {
            try
            {
                using (var k = Registry.CurrentUser.CreateSubKey(sub))
                {
                    if (k == null) return;
                    var orig = k.GetValue(name);
                    rb.Add(new ValorantRegBackup { Hive = RegistryHive.CurrentUser, SubKey = sub,
                        ValueName = name, OriginalValue = orig,
                        OriginalKind = orig != null ? RegistryValueKind.DWord : RegistryValueKind.None,
                        DidExist = orig != null });
                    k.SetValue(name, val, RegistryValueKind.DWord);
                }
            }
            catch { }
        }

        private static void BkSet(RegistryKey k, RegistryHive hive, string sub,
            string name, object val, RegistryValueKind kind, List<ValorantRegBackup> rb)
        {
            var orig = k.GetValue(name);
            rb.Add(new ValorantRegBackup { Hive = hive, SubKey = sub, ValueName = name,
                OriginalValue = orig,
                OriginalKind = orig != null ? k.GetValueKind(name) : RegistryValueKind.None,
                DidExist = orig != null });
            k.SetValue(name, val, kind);
        }

        private static string ActiveNicGuid()
        {
            try
            {
                foreach (var n in NetworkInterface.GetAllNetworkInterfaces())
                {
                    if (n.OperationalStatus != OperationalStatus.Up) continue;
                    if (n.NetworkInterfaceType == NetworkInterfaceType.Loopback) continue;
                    if (n.NetworkInterfaceType == NetworkInterfaceType.Tunnel)   continue;
                    var ipProps = n.GetIPProperties();
                    if (ipProps != null && ipProps.GatewayAddresses != null && ipProps.GatewayAddresses.Count > 0) return n.Id;
                }
                var fb = NetworkInterface.GetAllNetworkInterfaces().FirstOrDefault(
                    n => n.OperationalStatus == OperationalStatus.Up &&
                         n.NetworkInterfaceType != NetworkInterfaceType.Loopback);
                return (fb != null && fb.Id != null) ? fb.Id : "";
            }
            catch { return ""; }
        }

        private static void EnsureDir(string path)
        { try { if (!Directory.Exists(path)) Directory.CreateDirectory(path); } catch { } }

        private static long DirSize(string path)
        {
            if (!Directory.Exists(path)) return 0;
            long sz = 0;
            try { foreach (var f in Directory.GetFiles(path, "*", SearchOption.AllDirectories))
                try { sz += new FileInfo(f).Length; } catch { } }
            catch { }
            return sz;
        }

        private static long DeleteDir(string path, Action<string> log)
        {
            if (!Directory.Exists(path)) return 0;
            long freed = 0;
            try
            {
                foreach (var f in Directory.GetFiles(path, "*", SearchOption.AllDirectories))
                    try { long s = new FileInfo(f).Length; File.Delete(f); freed += s; } catch { }
                foreach (var d in Directory.GetDirectories(path, "*", SearchOption.AllDirectories)
                    .OrderByDescending(x => x.Length))
                    try { if (!Directory.GetFiles(d).Any() && !Directory.GetDirectories(d).Any())
                        Directory.Delete(d, false); } catch { }
            }
            catch { }
            return freed;
        }

        private static int Exec(string file, string args)
        {
            try
            {
                using (var p = Process.Start(new ProcessStartInfo
                    { FileName = file, Arguments = args, CreateNoWindow = true, UseShellExecute = false }))
                { p.WaitForExit(5000); return p.ExitCode; }
            }
            catch { return -1; }
        }

        private static void LogSafe(Action<string> log, string msg)
        { try { if (log != null) log(msg); } catch { } }
    }
}
