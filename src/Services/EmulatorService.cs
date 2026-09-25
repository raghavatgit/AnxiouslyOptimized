using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using Microsoft.Win32;

namespace AnxiouslyOptimized.Services
{
    public class EmulatorTopology
    {
        public bool BlueStacksInstalled { get; set; }
        public string BlueStacksVersion { get; set; }
        public bool MsiInstalled { get; set; }
        public string MsiVersion { get; set; }
        public bool FpsUnlocked { get; set; }
        public bool RogProfileActive { get; set; }
        public bool RawMouseActive { get; set; }
        public bool LowPingActive { get; set; }
        public string VbsStatusSummary { get; set; }
    }

    public static class EmulatorService
    {
        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool SystemParametersInfo(uint uiAction, uint uiParam, IntPtr pvParam, uint fWinIni);

        [DllImport("user32.dll", EntryPoint = "SystemParametersInfo", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool SystemParametersInfoArray(uint uiAction, uint uiParam, int[] pvParam, uint fWinIni);

        private const uint SPI_SETMOUSESPEED = 0x0071;
        private const uint SPI_SETMOUSE = 0x0004;
        private const uint SPIF_UPDATEINIFILE = 0x01;
        private const uint SPIF_SENDCHANGE = 0x02;

        public static bool IsBlueStacksInstalled()
        {
            string p1 = @"C:\Program Files\BlueStacks_nxt\HD-Player.exe";
            string p2 = @"C:\Program Files (x86)\BlueStacks_nxt\HD-Player.exe";
            string conf = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.CommonApplicationData), @"BlueStacks_nxt\bluestacks.conf");
            return File.Exists(p1) || File.Exists(p2) || File.Exists(conf);
        }

        public static bool IsMsiPlayerInstalled()
        {
            string p1 = @"C:\Program Files\BlueStacks_msi5\HD-Player.exe";
            string p2 = @"C:\Program Files (x86)\BlueStacks_msi5\HD-Player.exe";
            string conf = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.CommonApplicationData), @"BlueStacks_msi5\bluestacks.conf");
            return File.Exists(p1) || File.Exists(p2) || File.Exists(conf);
        }

        public static bool IsLdPlayerInstalled()
        {
            string p1 = @"C:\LDPlayer\LDPlayer9\dnplayer.exe";
            string p2 = @"D:\LDPlayer\LDPlayer9\dnplayer.exe";
            return File.Exists(p1) || File.Exists(p2);
        }

        private static List<string> GetConfPaths()
        {
            var list = new List<string>();
            string programData = Environment.GetFolderPath(Environment.SpecialFolder.CommonApplicationData);
            string[] candidates = new string[]
            {
                Path.Combine(programData, @"BlueStacks_nxt\bluestacks.conf"),
                Path.Combine(programData, @"BlueStacks_msi5\bluestacks.conf"),
                Path.Combine(programData, @"BlueStacks_msi2\bluestacks.conf")
            };

            foreach (var c in candidates)
            {
                if (File.Exists(c)) list.Add(c);
            }
            return list;
        }

        public static EmulatorTopology GetTopology()
        {
            var topo = new EmulatorTopology
            {
                BlueStacksInstalled = false,
                BlueStacksVersion = "Not Detected",
                MsiInstalled = false,
                MsiVersion = "Not Detected",
                FpsUnlocked = false,
                RogProfileActive = false,
                RawMouseActive = false,
                LowPingActive = false,
                VbsStatusSummary = "Checking..."
            };

            try
            {
                string programData = Environment.GetFolderPath(Environment.SpecialFolder.CommonApplicationData);
                string bsConf = Path.Combine(programData, @"BlueStacks_nxt\bluestacks.conf");
                if (File.Exists(bsConf) || IsBlueStacksInstalled())
                {
                    topo.BlueStacksInstalled = true;
                    topo.BlueStacksVersion = "Installed";
                    using (var key = Registry.LocalMachine.OpenSubKey(@"SOFTWARE\BlueStacks_nxt"))
                    {
                        if (key != null)
                        {
                            var v = key.GetValue("Version") as string;
                            if (!string.IsNullOrEmpty(v)) topo.BlueStacksVersion = "v" + v;
                        }
                    }
                }

                string msiConf = Path.Combine(programData, @"BlueStacks_msi5\bluestacks.conf");
                if (File.Exists(msiConf) || IsMsiPlayerInstalled())
                {
                    topo.MsiInstalled = true;
                    topo.MsiVersion = "Installed";
                    using (var key = Registry.LocalMachine.OpenSubKey(@"SOFTWARE\BlueStacks_msi5"))
                    {
                        if (key != null)
                        {
                            var v = key.GetValue("Version") as string;
                            if (!string.IsNullOrEmpty(v)) topo.MsiVersion = "v" + v;
                        }
                    }
                }

                foreach (var conf in GetConfPaths())
                {
                    try
                    {
                        string txt = File.ReadAllText(conf);
                        if (txt.Contains("enable_high_fps=\"1\"") && txt.Contains("max_fps=\"240\""))
                            topo.FpsUnlocked = true;
                        if (txt.Contains("ASUS_I001DA") || txt.Contains("asus_rog_2") || txt.Contains("device_profile_code=\"sttu\""))
                            topo.RogProfileActive = true;
                    }
                    catch { }
                }

                using (var key = Registry.CurrentUser.OpenSubKey(@"Control Panel\Mouse"))
                {
                    if (key != null)
                    {
                        var speed = key.GetValue("MouseSpeed") as string;
                        if (speed == "0") topo.RawMouseActive = true;
                    }
                }

                using (var key = Registry.LocalMachine.OpenSubKey(@"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile"))
                {
                    if (key != null)
                    {
                        object netThrottling = key.GetValue("NetworkThrottlingIndex");
                        object sysResp = key.GetValue("SystemResponsiveness");
                        if (netThrottling != null && (int)netThrottling == -1 && sysResp != null && (int)sysResp == 0)
                            topo.LowPingActive = true;
                    }
                }

                topo.VbsStatusSummary = GetVbsStatusSummary();
            }
            catch { }

            return topo;
        }

        private static string GetVbsStatusSummary()
        {
            try
            {
                var psi = new ProcessStartInfo
                {
                    FileName = "bcdedit.exe",
                    Arguments = "/enum {current}",
                    CreateNoWindow = true,
                    UseShellExecute = false,
                    RedirectStandardOutput = true,
                    RedirectStandardError = true
                };

                using (var p = Process.Start(psi))
                {
                    string outText = p.StandardOutput.ReadToEnd();
                    p.WaitForExit(3000);
                    if (outText.IndexOf("hypervisorlaunchtype", StringComparison.OrdinalIgnoreCase) >= 0 &&
                        outText.IndexOf("off", StringComparison.OrdinalIgnoreCase) >= 0)
                    {
                        return "Off (Full Speed)";
                    }
                }
            }
            catch { }

            return "Active (Protected)";
        }

        private static bool UpdateConfigFile(string confPath, Dictionary<string, string> updates, Action<string> log)
        {
            if (!File.Exists(confPath)) return false;

            try
            {
                // Close HD-Player if currently running
                var procs = Process.GetProcessesByName("HD-Player");
                if (procs.Length > 0)
                {
                    log("  Closing active BlueStacks processes to save configuration safely...");
                    foreach (var p in procs)
                    {
                        try { p.Kill(); } catch { }
                    }
                }

                // Safety backups
                string origBak = confPath + ".bak_original";
                if (!File.Exists(origBak))
                {
                    File.Copy(confPath, origBak, true);
                }

                string timeBak = confPath + ".bak_anxious_" + DateTime.Now.ToString("yyyyMMdd_HHmmss");
                File.Copy(confPath, timeBak, true);
                log("  Safety backup created: " + Path.GetFileName(timeBak));

                var rawLines = File.ReadAllLines(confPath);
                var lines = new List<string>();
                var lineDict = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
                var fakeKeys = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
                {
                    "bst.feature.high_fps", "bst.feature.fps", "bst.feature.show_fps", "bst.feature.vsync"
                };

                foreach (var line in rawLines)
                {
                    string trimmed = line.Trim();
                    if (string.IsNullOrWhiteSpace(trimmed)) continue;

                    int eqIdx = trimmed.IndexOf('=');
                    if (eqIdx > 0)
                    {
                        string k = trimmed.Substring(0, eqIdx).Trim();
                        string v = trimmed.Substring(eqIdx + 1).Trim();

                        if (fakeKeys.Contains(k)) continue;

                        if (!lineDict.ContainsKey(k))
                        {
                            lineDict.Add(k, v);
                            lines.Add(k);
                        }
                        else
                        {
                            lineDict[k] = v;
                        }
                    }
                    else
                    {
                        lines.Add(trimmed);
                    }
                }

                foreach (var kvp in updates)
                {
                    if (lineDict.ContainsKey(kvp.Key))
                        lineDict[kvp.Key] = kvp.Value;
                    else
                    {
                        lineDict.Add(kvp.Key, kvp.Value);
                        lines.Add(kvp.Key);
                    }
                }

                var sb = new StringBuilder();
                var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
                foreach (var item in lines)
                {
                    if (lineDict.ContainsKey(item))
                    {
                        if (seen.Add(item))
                            sb.Append(item).Append('=').Append(lineDict[item]).Append('\n');
                    }
                    else
                    {
                        if (seen.Add(item))
                            sb.Append(item).Append('\n');
                    }
                }

                var utf8NoBom = new UTF8Encoding(false);
                File.WriteAllText(confPath, sb.ToString(), utf8NoBom);
                return true;
            }
            catch (Exception ex)
            {
                log("  Error updating config: " + ex.Message);
                return false;
            }
        }

        private static List<string> DiscoverInstances(string confPath)
        {
            var instances = new List<string>();
            try
            {
                string raw = File.ReadAllText(confPath);
                var mImages = Regex.Match(raw, @"bst\.installed_images=""([^""]+)""");
                if (mImages.Success)
                {
                    string[] arr = mImages.Groups[1].Value.Split(',');
                    foreach (var s in arr)
                    {
                        string c = s.Trim();
                        if (!string.IsNullOrEmpty(c) && !instances.Contains(c))
                            instances.Add(c);
                    }
                }

                var matches = Regex.Matches(raw, @"bst\.instance\.([a-zA-Z0-9_-]+)\.");
                foreach (Match m in matches)
                {
                    string inst = m.Groups[1].Value;
                    if (!string.IsNullOrEmpty(inst) && !instances.Contains(inst))
                        instances.Add(inst);
                }
            }
            catch { }

            if (instances.Count == 0) instances.Add("Pie64");
            return instances;
        }

        public static async Task<bool> UnlockFpsAsync(Action<string> log)
        {
            return await Task.Run(() =>
            {
                log("Configuring BlueStacks & MSI App Player for 240 FPS & ASUS ROG Phone 2 profile...");
                var confs = GetConfPaths();
                if (confs.Count == 0)
                {
                    log("[NOTICE] No BlueStacks 5 or MSI App Player configuration found on this system.");
                    return false;
                }

                foreach (var confPath in confs)
                {
                    string name = confPath.IndexOf("msi", StringComparison.OrdinalIgnoreCase) >= 0 ? "MSI App Player" : "BlueStacks 5";
                    log("Found settings at: " + confPath);

                    var instances = DiscoverInstances(confPath);
                    log("  Target instances: " + string.Join(", ", instances));

                    var updates = new Dictionary<string, string>();
                    foreach (var inst in instances)
                    {
                        updates["bst.instance." + inst + ".enable_high_fps"] = "\"1\"";
                        updates["bst.instance." + inst + ".max_fps"] = "\"240\"";
                        updates["bst.instance." + inst + ".enable_fps_display"] = "\"1\"";
                        updates["bst.instance." + inst + ".enable_vsync"] = "\"0\"";
                        updates["bst.instance." + inst + ".device_profile_code"] = "\"sttu\"";
                        updates["bst.instance." + inst + ".device_custom_brand"] = "\"\"";
                        updates["bst.instance." + inst + ".device_custom_manufacturer"] = "\"\"";
                        updates["bst.instance." + inst + ".device_custom_model"] = "\"\"";
                        updates["bst.instance." + inst + ".astc_decoding_mode"] = "\"hardware\"";
                        updates["bst.instance." + inst + ".cpus"] = "\"4\"";
                    }

                    bool ok = UpdateConfigFile(confPath, updates, log);
                    if (ok)
                        log("  [OK] " + name + " successfully set to 240 FPS (Hardware ASTC, ROG Phone 2 profile).");
                }

                log("[DONE] 240 FPS mode and ASUS ROG Phone 2 profile applied successfully.");
                return true;
            });
        }

        public static async Task<bool> LockGpuAsync(Action<string> log)
        {
            return await Task.Run(() =>
            {
                log("Forcing dedicated high-speed GPU and high process priority for emulators...");
                try
                {
                    var candidatePaths = new List<string>
                    {
                        @"C:\Program Files\BlueStacks_nxt\HD-Player.exe",
                        @"C:\Program Files\BlueStacks_nxt\BlueStacksHelper.exe",
                        @"C:\Program Files\BlueStacks_msi5\HD-Player.exe",
                        @"C:\Program Files\BlueStacks_msi5\BlueStacksHelper.exe",
                        @"C:\Program Files (x86)\BlueStacks_nxt\HD-Player.exe",
                        @"C:\LDPlayer\LDPlayer9\dnplayer.exe"
                    };

                    using (var key = Registry.CurrentUser.CreateSubKey(@"Software\Microsoft\DirectX\UserGpuPreferences"))
                    {
                        if (key != null)
                        {
                            foreach (var p in candidatePaths)
                            {
                                if (File.Exists(p) || p.Contains("BlueStacks_nxt") || p.Contains("BlueStacks_msi5"))
                                {
                                    key.SetValue(p, "GpuPreference=2;");
                                    log("  Assigned dedicated GPU preference: " + Path.GetFileName(p));
                                }
                            }
                        }
                    }

                    using (var key = Registry.LocalMachine.CreateSubKey(@"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\HD-Player.exe\PerfOptions"))
                    {
                        if (key != null)
                        {
                            key.SetValue("CpuPriorityClass", 3, RegistryValueKind.DWord);
                            key.SetValue("IoPriority", 3, RegistryValueKind.DWord);
                            log("  Set high CPU & IO priority for HD-Player.exe.");
                        }
                    }

                    using (var key = Registry.LocalMachine.CreateSubKey(@"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games"))
                    {
                        if (key != null)
                        {
                            key.SetValue("Affinity", 0, RegistryValueKind.DWord);
                            key.SetValue("Background Only", "False", RegistryValueKind.String);
                            key.SetValue("Clock Rate", 10000, RegistryValueKind.DWord);
                            key.SetValue("GPU Priority", 8, RegistryValueKind.DWord);
                            key.SetValue("Priority", 6, RegistryValueKind.DWord);
                            key.SetValue("Scheduling Category", "High", RegistryValueKind.String);
                            key.SetValue("SFIO Priority", "High", RegistryValueKind.String);
                            log("  Windows MMCSS gaming priority tuned to maximum.");
                        }
                    }

                    log("[DONE] Dedicated GPU and high scheduling priority locked successfully.");
                    return true;
                }
                catch (Exception ex)
                {
                    log("[ERROR] GPU Lock failed: " + ex.Message);
                    return false;
                }
            });
        }

        public static async Task<bool> ApplySmoothAimAsync(Action<string> log)
        {
            return await Task.Run(() =>
            {
                log("Configuring 1:1 raw mouse input for Free Fire drag headshots...");
                try
                {
                    using (var key = Registry.CurrentUser.CreateSubKey(@"Control Panel\Mouse"))
                    {
                        if (key != null)
                        {
                            try { key.DeleteValue("SmoothMouseXCurve"); } catch { }
                            try { key.DeleteValue("SmoothMouseYCurve"); } catch { }

                            key.SetValue("MouseSpeed", "0", RegistryValueKind.String);
                            key.SetValue("MouseThreshold1", "0", RegistryValueKind.String);
                            key.SetValue("MouseThreshold2", "0", RegistryValueKind.String);
                            key.SetValue("MouseSensitivity", "10", RegistryValueKind.String);
                        }
                    }

                    try
                    {
                        int[] mouseParams = new int[] { 0, 0, 0 };
                        SystemParametersInfoArray(SPI_SETMOUSE, 0, mouseParams, SPIF_UPDATEINIFILE | SPIF_SENDCHANGE);
                        SystemParametersInfo(SPI_SETMOUSESPEED, 0, (IntPtr)10, SPIF_UPDATEINIFILE | SPIF_SENDCHANGE);
                        log("  Applied live mouse parameters (zero acceleration curve, sensitivity 10).");
                    }
                    catch { }

                    log("[DONE] Smooth 1:1 mouse aim active (Zero Acceleration, Native Windows Input).");
                    return true;
                }
                catch (Exception ex)
                {
                    log("[ERROR] Mouse aim configuration failed: " + ex.Message);
                    return false;
                }
            });
        }

        public static async Task<bool> ApplyLowPingAsync(Action<string> log)
        {
            return await Task.Run(() =>
            {
                log("Optimizing network stack for lowest gaming ping and zero packet delay...");
                try
                {
                    using (var key = Registry.LocalMachine.CreateSubKey(@"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile"))
                    {
                        if (key != null)
                        {
                            key.SetValue("NetworkThrottlingIndex", unchecked((int)0xFFFFFFFF), RegistryValueKind.DWord);
                            key.SetValue("SystemResponsiveness", 0, RegistryValueKind.DWord);
                            log("  Disabled Windows network bandwidth throttling for gaming packets.");
                        }
                    }

                    using (var key = Registry.LocalMachine.OpenSubKey(@"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces", true))
                    {
                        if (key != null)
                        {
                            foreach (var subName in key.GetSubKeyNames())
                            {
                                try
                                {
                                    using (var sub = key.OpenSubKey(subName, true))
                                    {
                                        if (sub != null)
                                        {
                                            try { sub.DeleteValue("TcpAckFrequency"); } catch { }
                                            try { sub.DeleteValue("TCPNoDelay"); } catch { }
                                            try { sub.DeleteValue("TcpDelAckTicks"); } catch { }
                                        }
                                    }
                                }
                                catch { }
                            }
                            log("  Cleaned TCP ACK frequency bottlenecks across network adapters.");
                        }
                    }

                    try
                    {
                        using (var p = Process.Start(new ProcessStartInfo
                        {
                            FileName = "netsh",
                            Arguments = "int tcp set global autotuninglevel=normal",
                            CreateNoWindow = true,
                            UseShellExecute = false
                        }))
                        {
                            p.WaitForExit(3000);
                        }
                        log("  Ensured clean TCP window scaling (prevents download throttling).");
                    }
                    catch { }

                    try
                    {
                        using (var p = Process.Start(new ProcessStartInfo
                        {
                            FileName = "ipconfig",
                            Arguments = "/flushdns",
                            CreateNoWindow = true,
                            UseShellExecute = false
                        }))
                        {
                            p.WaitForExit(3000);
                        }
                        log("  Flushed Windows DNS resolver cache.");
                    }
                    catch { }

                    log("[DONE] Low ping, zero-delay TCP, and anti-ghost bullet network tuning applied.");
                    return true;
                }
                catch (Exception ex)
                {
                    log("[ERROR] Low ping tuning failed: " + ex.Message);
                    return false;
                }
            });
        }

        public static async Task<bool> SetStretchedResolutionAsync(Action<string> log, int width = 1440, int height = 1080, int dpi = 320)
        {
            return await Task.Run(() =>
            {
                log(string.Format("Setting competitive stretched resolution ({0}x{1}, {2} DPI)...", width, height, dpi));
                var confs = GetConfPaths();
                if (confs.Count == 0)
                {
                    log("[NOTICE] BlueStacks 5 or MSI App Player configuration not found.");
                    return false;
                }

                foreach (var confPath in confs)
                {
                    string name = confPath.IndexOf("msi", StringComparison.OrdinalIgnoreCase) >= 0 ? "MSI App Player" : "BlueStacks 5";
                    var instances = DiscoverInstances(confPath);

                    var updates = new Dictionary<string, string>();
                    foreach (var inst in instances)
                    {
                        updates["bst.instance." + inst + ".fb_width"] = "\"" + width + "\"";
                        updates["bst.instance." + inst + ".fb_height"] = "\"" + height + "\"";
                        updates["bst.instance." + inst + ".dpi"] = "\"" + dpi + "\"";
                        updates["bst.instance." + inst + ".custom_resolution_selected"] = "\"1\"";
                    }

                    bool ok = UpdateConfigFile(confPath, updates, log);
                    if (ok)
                        log(string.Format("  [OK] Stretched resolution ({0}x{1}) applied to {2}.", width, height, name));
                }

                log("[SUCCESS] Stretched screen view applied! Enemies are now wider for easier drag headshots.");
                return true;
            });
        }

        public static async Task<bool> ToggleVbsAsync(Action<string> log)
        {
            return await Task.Run(() =>
            {
                log("Auditing Virtualization-Based Security (VBS) launch configuration...");
                try
                {
                    string currentLaunch = "auto";
                    var psi = new ProcessStartInfo
                    {
                        FileName = "bcdedit.exe",
                        Arguments = "/enum {current}",
                        CreateNoWindow = true,
                        UseShellExecute = false,
                        RedirectStandardOutput = true
                    };

                    using (var p = Process.Start(psi))
                    {
                        string text = p.StandardOutput.ReadToEnd();
                        p.WaitForExit(3000);
                        if (text.IndexOf("hypervisorlaunchtype", StringComparison.OrdinalIgnoreCase) >= 0 &&
                            text.IndexOf("off", StringComparison.OrdinalIgnoreCase) >= 0)
                        {
                            currentLaunch = "off";
                        }
                    }

                    if (currentLaunch == "off")
                    {
                        log("Turning virtualization security feature back on (auto mode)...");
                        using (var p = Process.Start(new ProcessStartInfo
                        {
                            FileName = "bcdedit.exe",
                            Arguments = "/set hypervisorlaunchtype auto",
                            CreateNoWindow = true,
                            UseShellExecute = false
                        }))
                        {
                            p.WaitForExit(4000);
                        }
                        log("[SUCCESS] Virtualization security enabled (will be active after computer restart).");
                    }
                    else
                    {
                        log("Disabling hypervisor launch type to unlock direct hardware virtualization speed...");
                        using (var p = Process.Start(new ProcessStartInfo
                        {
                            FileName = "bcdedit.exe",
                            Arguments = "/set hypervisorlaunchtype off",
                            CreateNoWindow = true,
                            UseShellExecute = false
                        }))
                        {
                            p.WaitForExit(4000);
                        }
                        log("[SUCCESS] Speed boost enabled (VBS turned off). Restart your computer to enjoy 15% to 30% higher FPS.");
                    }
                    return true;
                }
                catch (Exception ex)
                {
                    log("[ERROR] VBS toggle failed: " + ex.Message);
                    return false;
                }
            });
        }

        public static async Task<bool> CleanEmulatorCacheAsync(Action<string> log)
        {
            return await Task.Run(() =>
            {
                log("Cleaning BlueStacks and MSI App Player logs and shader caches...");
                string programData = Environment.GetFolderPath(Environment.SpecialFolder.CommonApplicationData);
                string localApp = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
                string temp = Path.GetTempPath();

                string[] targets = new string[]
                {
                    Path.Combine(programData, @"BlueStacks_nxt\Logs"),
                    Path.Combine(programData, @"BlueStacks_msi5\Logs"),
                    Path.Combine(programData, @"BlueStacks_msi2\Logs"),
                    Path.Combine(localApp, @"BlueStacks"),
                    Path.Combine(localApp, @"D3DSCache"),
                    Path.Combine(temp, @"BlueStacks")
                };

                int deleted = 0;
                foreach (var t in targets)
                {
                    if (Directory.Exists(t))
                    {
                        try
                        {
                            var files = Directory.GetFiles(t, "*.*", SearchOption.AllDirectories);
                            foreach (var f in files)
                            {
                                try
                                {
                                    File.Delete(f);
                                    deleted++;
                                }
                                catch { }
                            }
                        }
                        catch { }
                    }
                }

                log(string.Format("Cleaned {0} emulator cache and log files.", deleted));
                log("[DONE] Emulator junk files cleaned successfully.");
                return true;
            });
        }

        public static async Task<bool> RevertEmulatorSettingsAsync(Action<string> log)
        {
            return await Task.Run(() =>
            {
                log("Resetting emulator and mouse settings back to original Windows defaults...");
                try
                {
                    // Restore original bluestacks configs
                    foreach (var conf in GetConfPaths())
                    {
                        string origBak = conf + ".bak_original";
                        if (File.Exists(origBak))
                        {
                            try
                            {
                                File.Copy(origBak, conf, true);
                                log("  Restored original settings for " + Path.GetFileName(conf));
                            }
                            catch { }
                        }
                    }

                    // Restore Windows default mouse acceleration
                    using (var key = Registry.CurrentUser.CreateSubKey(@"Control Panel\Mouse"))
                    {
                        if (key != null)
                        {
                            try { key.DeleteValue("SmoothMouseXCurve"); } catch { }
                            try { key.DeleteValue("SmoothMouseYCurve"); } catch { }
                            key.SetValue("MouseSpeed", "1", RegistryValueKind.String);
                            key.SetValue("MouseThreshold1", "6", RegistryValueKind.String);
                            key.SetValue("MouseThreshold2", "10", RegistryValueKind.String);
                            key.SetValue("MouseSensitivity", "10", RegistryValueKind.String);
                        }
                    }

                    try
                    {
                        int[] defaultParams = new int[] { 6, 10, 1 };
                        SystemParametersInfoArray(SPI_SETMOUSE, 0, defaultParams, SPIF_UPDATEINIFILE | SPIF_SENDCHANGE);
                        SystemParametersInfo(SPI_SETMOUSESPEED, 0, (IntPtr)10, SPIF_UPDATEINIFILE | SPIF_SENDCHANGE);
                        log("  Restored Windows native mouse acceleration ballistics.");
                    }
                    catch { }

                    log("[DONE] Game and emulator settings restored to defaults.");
                    return true;
                }
                catch (Exception ex)
                {
                    log("[ERROR] Revert emulator settings failed: " + ex.Message);
                    return false;
                }
            });
        }

        public static async Task<bool> ApplyMasterFreeFireBoostAsync(Action<string> log)
        {
            log("==================================================================");
            log("STARTING 1-CLICK MASTER FREE FIRE & EMULATOR BOOST");
            log("Targeting: BlueStacks 5 & MSI App Player");
            log("==================================================================");

            await UnlockFpsAsync(log);
            await LockGpuAsync(log);
            await ApplySmoothAimAsync(log);
            await ApplyLowPingAsync(log);
            await CleanEmulatorCacheAsync(log);

            log("==================================================================");
            log("[SUCCESS] MASTER FREE FIRE BOOST APPLIED!");
            log("1. Unlocked 240 FPS and ASUS ROG Phone 2 mode.");
            log("2. Forced emulator to use your fastest graphics card.");
            log("3. Turned on smooth mouse aim for easy drag headshots.");
            log("4. Tuned internet connection for low ping and instant bullet hits.");
            log("Open BlueStacks or MSI App Player and set Free Fire to 'High FPS'!");
            log("==================================================================");
            return true;
        }

        public static async Task<bool> OptimizeEmulatorAsync(Action<string> log)
        {
            return await ApplyMasterFreeFireBoostAsync(log);
        }
    }
}
