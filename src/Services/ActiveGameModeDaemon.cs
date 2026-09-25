using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using System.Web.Script.Serialization;

namespace AnxiouslyOptimized.Services
{
    public class GameModeStatusEventArgs : EventArgs
    {
        public bool IsGameActive { get; set; }
        public string GameName { get; set; }
        public int ProcessId { get; set; }
        public string Summary { get; set; }
        public DateTime Timestamp { get; set; }
    }

    public static class ActiveGameModeDaemon
    {
        #region Win32 Native Interop
        [DllImport("winmm.dll", EntryPoint = "timeBeginPeriod", SetLastError = true)]
        private static extern uint TimeBeginPeriod(uint uMilliseconds);

        [DllImport("winmm.dll", EntryPoint = "timeEndPeriod", SetLastError = true)]
        private static extern uint TimeEndPeriod(uint uMilliseconds);

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool SetProcessAffinityMask(IntPtr hProcess, UIntPtr dwProcessAffinityMask);

        [StructLayout(LayoutKind.Sequential)]
        private struct PROCESS_POWER_THROTTLING_STATE
        {
            public uint Version;
            public uint ControlMask;
            public uint StateMask;
        }

        private const int ProcessPowerThrottling = 4;
        private const uint PROCESS_POWER_THROTTLING_CURRENT_VERSION = 1;
        private const uint PROCESS_POWER_THROTTLING_EXECUTION_SPEED = 0x1;

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool SetProcessInformation(
            IntPtr hProcess,
            int processInformationClass,
            ref PROCESS_POWER_THROTTLING_STATE processInformation,
            uint processInformationSize);
        #endregion

        #region Configuration & State
        private static Timer _monitorTimer;
        private static readonly object _lock = new object();
        private static bool _isEnabled = false;
        private static bool _timerPeriodActive = false;
        private static string _originalPowerSchemeGuid = null;
        private static int _activeGamePid = 0;
        private static string _activeGameName = null;
        private static readonly Dictionary<int, ProcessPriorityClass> _throttledBackgroundProcesses = new Dictionary<int, ProcessPriorityClass>();

        // Default monitored gaming and emulator executables
        private static readonly HashSet<string> _monitoredProcesses = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
        {
            // BlueStacks & MSI App Player
            "HD-Player",
            "HD-Player.exe",
            "BlueStacks",
            "BlueStacks.exe",
            "HD-Agent",
            // LDPlayer & Nox & MuMu
            "dnplayer",
            "dnplayer.exe",
            "LdBoxHeadless",
            "Nox",
            "Nox.exe",
            "NoxVMHandle",
            "MuMuPlayer",
            "NemuPlayer",
            // Free Fire / PC wrappers
            "FreeFire",
            "FreeFireMax",
            // Popular Competitive PC Games
            "cs2",
            "valorant",
            "VALORANT-Win64-Shipping",
            "FortniteClient-Win64-Shipping",
            "r5apex",
            "Overwatch",
            "RainbowSix",
            "GTA5",
            "League of Legends",
            "RobloxPlayerBeta"
        };

        // Processes to lower priority for during active gameplay
        private static readonly HashSet<string> _backgroundThrottleTargets = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
        {
            "chrome",
            "msedge",
            "firefox",
            "discord",
            "spotify",
            "onedrive",
            "dropbox",
            "steamwebhelper",
            "epicgameslauncher"
        };

        public static bool IsEnabled
        {
            get { return _isEnabled; }
        }

        public static bool IsGameActive
        {
            get { return _activeGamePid > 0; }
        }

        public static string ActiveGameName
        {
            get { return _activeGameName ?? "None"; }
        }

        public static int ActiveGamePid
        {
            get { return _activeGamePid; }
        }

        // Toggles
        public static bool EnablePCorePinning { get; set; }
        public static bool EnableHighPriority { get; set; }
        public static bool EnablePowerSchemeSwitch { get; set; }
        public static bool Enable1msTimerResolution { get; set; }
        public static bool EnableBackgroundThrottling { get; set; }

        public static event Action<GameModeStatusEventArgs> StatusChanged;
        public static event Action<string> LogMessage;

        static ActiveGameModeDaemon()
        {
            EnablePCorePinning = true;
            EnableHighPriority = true;
            EnablePowerSchemeSwitch = true;
            Enable1msTimerResolution = true;
            EnableBackgroundThrottling = false; // Safe default
            LoadCustomTargets();
        }
        #endregion

        #region Public Daemon Lifecycle
        public static void StartDaemon()
        {
            lock (_lock)
            {
                if (_isEnabled) return;
                _isEnabled = true;

                // Monitor loop runs every 1200ms with negligible CPU impact
                _monitorTimer = new Timer(OnMonitorTick, null, 500, 1200);
                Log("Active Game Mode Daemon started (Monitoring emulators and competitive games).");
                NotifyStatus(false, null, 0, "Daemon Running (Standby)");
            }
        }

        public static void StopDaemon()
        {
            lock (_lock)
            {
                if (!_isEnabled) return;
                _isEnabled = false;

                if (_monitorTimer != null)
                {
                    _monitorTimer.Dispose();
                    _monitorTimer = null;
                }

                // If a game was currently active, revert all interventions
                if (_activeGamePid > 0)
                {
                    RevertGameInterventions();
                }

                Log("Active Game Mode Daemon stopped.");
                NotifyStatus(false, null, 0, "Daemon Stopped");
            }
        }

        public static void AddMonitoredProcess(string processName)
        {
            if (string.IsNullOrEmpty(processName)) return;
            string clean = processName.Trim();
            if (clean.EndsWith(".exe", StringComparison.OrdinalIgnoreCase))
                clean = clean.Substring(0, clean.Length - 4);

            lock (_lock)
            {
                _monitoredProcesses.Add(clean);
                _monitoredProcesses.Add(clean + ".exe");
                SaveCustomTargets();
            }
            Log(string.Format("Added '{0}' to Game Mode monitoring list.", clean));
        }

        public static List<string> GetMonitoredProcesses()
        {
            lock (_lock)
            {
                return _monitoredProcesses.Where(p => !p.EndsWith(".exe", StringComparison.OrdinalIgnoreCase)).OrderBy(p => p).ToList();
            }
        }
        #endregion

        #region Periodic Monitor Tick
        private static void OnMonitorTick(object state)
        {
            try
            {
                if (!_isEnabled) return;

                // Check if current active game process is still alive
                if (_activeGamePid > 0)
                {
                    bool stillRunning = false;
                    try
                    {
                        var proc = Process.GetProcessById(_activeGamePid);
                        if (!proc.HasExited)
                        {
                            stillRunning = true;
                        }
                    }
                    catch
                    {
                        stillRunning = false;
                    }

                    if (!stillRunning)
                    {
                        // Game exited
                        lock (_lock)
                        {
                            Log(string.Format("Target game '{0}' (PID {1}) has exited. Reverting optimizations...", _activeGameName, _activeGamePid));
                            RevertGameInterventions();
                        }
                    }
                    return;
                }

                // Search for newly launched game/emulator
                Process foundGame = null;
                Process[] allProcs = Process.GetProcesses();

                for (int i = 0; i < allProcs.Length; i++)
                {
                    Process p = allProcs[i];
                    try
                    {
                        if (_monitoredProcesses.Contains(p.ProcessName))
                        {
                            foundGame = p;
                            break;
                        }
                    }
                    catch { }
                }

                if (foundGame != null)
                {
                    lock (_lock)
                    {
                        ApplyGameInterventions(foundGame);
                    }
                }
            }
            catch (Exception ex)
            {
                // Silent fail-safe to protect game loop
                Debug.WriteLine("GameMode daemon tick exception: " + ex.Message);
            }
        }
        #endregion

        #region Optimization Interventions
        private static void ApplyGameInterventions(Process gameProc)
        {
            try
            {
                _activeGamePid = gameProc.Id;
                _activeGameName = gameProc.ProcessName;

                var summary = new StringBuilder();
                Log(string.Format("ACTIVE GAME DETECTED: '{0}' (PID: {1}). Triggering Game Mode Optimizations...", _activeGameName, _activeGamePid));

                // 1. Windows 1ms High-Precision Timer Resolution
                if (Enable1msTimerResolution && !_timerPeriodActive)
                {
                    uint res = TimeBeginPeriod(1);
                    if (res == 0) // TIMERR_NOERROR
                    {
                        _timerPeriodActive = true;
                        summary.Append("1ms Timer | ");
                        Log("  [GameMode] Locked Windows Global Timer Resolution to 1.0ms (Low Input Latency).");
                    }
                }

                // 2. CPU Priority Elevation to High
                if (EnableHighPriority)
                {
                    try
                    {
                        gameProc.PriorityClass = ProcessPriorityClass.High;
                        summary.Append("High Priority | ");
                        Log(string.Format("  [GameMode] Boosted '{0}' Process Priority to HIGH.", _activeGameName));
                    }
                    catch (Exception ex)
                    {
                        Log("  [GameMode] Could not set High priority: " + ex.Message);
                    }
                }

                // 3. Performance-Core (P-Core) Affinity Pinning
                if (EnablePCorePinning)
                {
                    try
                    {
                        long mask = CalculateOptimalPerformanceAffinityMask();
                        if (mask > 0)
                        {
                            SetProcessAffinityMask(gameProc.Handle, new UIntPtr((ulong)mask));
                            summary.Append(string.Format("P-Cores Pin (0x{0:X}) | ", mask));
                            Log(string.Format("  [GameMode] Pinned '{0}' threads to Performance Cores (Affinity Mask: 0x{1:X}).", _activeGameName, mask));
                        }
                    }
                    catch (Exception ex)
                    {
                        Log("  [GameMode] Affinity pinning notice: " + ex.Message);
                    }
                }

                // 4. Disable Windows Power Throttling for the game
                try
                {
                    var throttlingState = new PROCESS_POWER_THROTTLING_STATE
                    {
                        Version = PROCESS_POWER_THROTTLING_CURRENT_VERSION,
                        ControlMask = PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
                        StateMask = 0 // 0 = Disable power throttling
                    };

                    bool ptResult = SetProcessInformation(
                        gameProc.Handle,
                        ProcessPowerThrottling,
                        ref throttlingState,
                        (uint)Marshal.SizeOf(throttlingState));

                    if (ptResult)
                    {
                        summary.Append("Power Throttling Off | ");
                        Log(string.Format("  [GameMode] Disabled Windows Power Throttling on '{0}'.", _activeGameName));
                    }
                }
                catch { }

                // 5. Auto Power Scheme Switch
                if (EnablePowerSchemeSwitch)
                {
                    SwitchToGamingPowerScheme();
                    summary.Append("Ultimate Power Plan | ");
                }

                // 6. Background Process Throttling
                if (EnableBackgroundThrottling)
                {
                    ThrottleBackgroundApps();
                    summary.Append("Background Throttled | ");
                }

                string finalSummary = summary.ToString().TrimEnd(' ', '|');
                if (string.IsNullOrEmpty(finalSummary)) finalSummary = "Active";

                NotifyStatus(true, _activeGameName, _activeGamePid, finalSummary);
            }
            catch (Exception ex)
            {
                Log("Error during GameMode application: " + ex.Message);
            }
        }

        private static void RevertGameInterventions()
        {
            try
            {
                // 1. Release 1ms timer
                if (_timerPeriodActive)
                {
                    TimeEndPeriod(1);
                    _timerPeriodActive = false;
                    Log("  [GameMode] Restored Windows standard timer resolution.");
                }

                // 2. Restore Power Plan
                if (EnablePowerSchemeSwitch && !string.IsNullOrEmpty(_originalPowerSchemeGuid))
                {
                    RestoreOriginalPowerScheme();
                }

                // 3. Restore background process priorities
                if (_throttledBackgroundProcesses.Count > 0)
                {
                    RestoreBackgroundApps();
                }

                _activeGamePid = 0;
                _activeGameName = null;

                NotifyStatus(false, null, 0, "Daemon Running (Standby)");
                Log("All Game Mode optimizations cleanly reverted. Standing by for next game.");
            }
            catch (Exception ex)
            {
                Log("Error during GameMode revert: " + ex.Message);
            }
        }
        #endregion

        #region Helper Routines
        private static long CalculateOptimalPerformanceAffinityMask()
        {
            int coreCount = Environment.ProcessorCount;
            if (coreCount <= 1) return 1;

            // If system has 8 cores or fewer, use all cores
            if (coreCount <= 8)
            {
                return (1L << coreCount) - 1;
            }

            // On modern hybrid CPUs (Intel 12th/13th/14th Gen or AMD dual CCX):
            // Typically P-cores have hyperthreading (e.g. 8 cores = 16 logical threads),
            // and E-cores are single-threaded tacked on at the end.
            // Pin to first 16 threads (P-Cores) to avoid E-Core lag spikes.
            if (coreCount == 16 || coreCount == 20 || coreCount == 24 || coreCount == 32)
            {
                int pThreads = 16;
                if (coreCount == 20) pThreads = 12; // E.g. 6P + 8E = 12P threads + 8E threads
                if (coreCount == 16) pThreads = 16; // 8 cores with HT or 8P + 8E
                return (1L << pThreads) - 1;
            }

            // General fallback: mask top 75% of logical processors
            int activeThreads = (int)(coreCount * 0.75);
            if (activeThreads < 4) activeThreads = coreCount;
            return (1L << activeThreads) - 1;
        }

        private static void SwitchToGamingPowerScheme()
        {
            try
            {
                // Capture current scheme GUID
                string currentScheme = RunCommandSilent("powercfg.exe", "/getactivescheme");
                var m = System.Text.RegularExpressions.Regex.Match(currentScheme, @"([a-f0-9\-]{36})", System.Text.RegularExpressions.RegexOptions.IgnoreCase);
                if (m.Success)
                {
                    _originalPowerSchemeGuid = m.Groups[1].Value;
                }

                // Activate Ultimate Performance or High Performance
                string ultimateGuid = "e9a42b02-d5df-448d-aa00-03f14749eb61";
                string res = RunCommandSilent("powercfg.exe", "/setactive " + ultimateGuid);
                if (res.IndexOf("error", StringComparison.OrdinalIgnoreCase) >= 0)
                {
                    // Fall back to High Performance
                    RunCommandSilent("powercfg.exe", "/setactive 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c");
                }
                Log("  [GameMode] Automatically switched power scheme to Ultimate / High Performance.");
            }
            catch { }
        }

        private static void RestoreOriginalPowerScheme()
        {
            try
            {
                if (!string.IsNullOrEmpty(_originalPowerSchemeGuid))
                {
                    RunCommandSilent("powercfg.exe", "/setactive " + _originalPowerSchemeGuid);
                    Log(string.Format("  [GameMode] Restored previous active power scheme ({0}).", _originalPowerSchemeGuid));
                    _originalPowerSchemeGuid = null;
                }
            }
            catch { }
        }

        private static void ThrottleBackgroundApps()
        {
            _throttledBackgroundProcesses.Clear();
            try
            {
                foreach (var p in Process.GetProcesses())
                {
                    try
                    {
                        if (_backgroundThrottleTargets.Contains(p.ProcessName))
                        {
                            _throttledBackgroundProcesses[p.Id] = p.PriorityClass;
                            p.PriorityClass = ProcessPriorityClass.BelowNormal;
                        }
                    }
                    catch { }
                }
                if (_throttledBackgroundProcesses.Count > 0)
                {
                    Log(string.Format("  [GameMode] Throttled {0} background processes to BelowNormal priority.", _throttledBackgroundProcesses.Count));
                }
            }
            catch { }
        }

        private static void RestoreBackgroundApps()
        {
            try
            {
                int restoredCount = 0;
                foreach (var kvp in _throttledBackgroundProcesses)
                {
                    try
                    {
                        var p = Process.GetProcessById(kvp.Key);
                        if (!p.HasExited)
                        {
                            p.PriorityClass = kvp.Value;
                            restoredCount++;
                        }
                    }
                    catch { }
                }
                _throttledBackgroundProcesses.Clear();
                if (restoredCount > 0)
                {
                    Log(string.Format("  [GameMode] Restored priority for {0} background processes.", restoredCount));
                }
            }
            catch { }
        }

        private static string RunCommandSilent(string fileName, string args)
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
                    p.WaitForExit(2000);
                    return output ?? string.Empty;
                }
            }
            catch { return string.Empty; }
        }

        private static void NotifyStatus(bool isGameActive, string gameName, int pid, string summary)
        {
            var handler = StatusChanged;
            if (handler != null)
            {
                handler(new GameModeStatusEventArgs
                {
                    IsGameActive = isGameActive,
                    GameName = gameName,
                    ProcessId = pid,
                    Summary = summary,
                    Timestamp = DateTime.Now
                });
            }
        }

        private static void Log(string msg)
        {
            var handler = LogMessage;
            if (handler != null)
            {
                handler(msg);
            }
        }

        private static void LoadCustomTargets()
        {
            try
            {
                string configPath = Path.Combine(
                    Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
                    "AnxiouslyOptimized",
                    "gamemode_config.json");

                if (!File.Exists(configPath)) return;

                string json = File.ReadAllText(configPath);
                var ser = new JavaScriptSerializer();
                var list = ser.Deserialize<List<string>>(json);
                if (list != null)
                {
                    foreach (var item in list)
                    {
                        if (!string.IsNullOrEmpty(item))
                        {
                            _monitoredProcesses.Add(item);
                            _monitoredProcesses.Add(item.EndsWith(".exe", StringComparison.OrdinalIgnoreCase) ? item : item + ".exe");
                        }
                    }
                }
            }
            catch { }
        }

        private static void SaveCustomTargets()
        {
            try
            {
                string dir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "AnxiouslyOptimized");
                if (!Directory.Exists(dir)) Directory.CreateDirectory(dir);

                string configPath = Path.Combine(dir, "gamemode_config.json");
                var ser = new JavaScriptSerializer();
                var list = _monitoredProcesses.Where(p => !p.EndsWith(".exe", StringComparison.OrdinalIgnoreCase)).ToList();
                File.WriteAllText(configPath, ser.Serialize(list));
            }
            catch { }
        }
        #endregion
    }
}
