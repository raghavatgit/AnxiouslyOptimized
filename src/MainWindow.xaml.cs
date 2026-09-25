using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Threading;
using AnxiouslyOptimized.Models;
using AnxiouslyOptimized.Services;
using System.Windows.Interop;

namespace AnxiouslyOptimized
{
    public partial class MainWindow : Window
    {
        [DllImport("psapi.dll")]
        private static extern int EmptyWorkingSet(IntPtr hwProc);

        [StructLayout(LayoutKind.Sequential)]
        public struct POINT
        {
            public int x;
            public int y;
            public POINT(int x, int y) { this.x = x; this.y = y; }
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct MINMAXINFO
        {
            public POINT ptReserved;
            public POINT ptMaxSize;
            public POINT ptMaxPosition;
            public POINT ptMinTrackSize;
            public POINT ptMaxTrackSize;
        }

        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Auto)]
        public class MONITORINFO
        {
            public int cbSize = Marshal.SizeOf(typeof(MONITORINFO));
            public RECT rcMonitor = new RECT();
            public RECT rcWork = new RECT();
            public int dwFlags = 0;
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct RECT
        {
            public int left;
            public int top;
            public int right;
            public int bottom;
        }

        [DllImport("user32.dll")]
        private static extern IntPtr MonitorFromWindow(IntPtr hwnd, uint dwFlags);

        [DllImport("user32.dll", CharSet = CharSet.Auto)]
        private static extern bool GetMonitorInfo(IntPtr hMonitor, MONITORINFO lpmi);

        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Auto)]
        private class MEMORYSTATUSEX
        {
            public uint dwLength;
            public uint dwMemoryLoad;
            public ulong ullTotalPhys;
            public ulong ullAvailPhys;
            public ulong ullTotalPageFile;
            public ulong ullAvailPageFile;
            public ulong ullTotalVirtual;
            public ulong ullAvailVirtual;
            public ulong ullAvailExtendedVirtual;

            public MEMORYSTATUSEX()
            {
                this.dwLength = (uint)Marshal.SizeOf(typeof(MEMORYSTATUSEX));
            }
        }

        [return: MarshalAs(UnmanagedType.Bool)]
        [DllImport("kernel32.dll", CharSet = CharSet.Auto, SetLastError = true)]
        private static extern bool GlobalMemoryStatusEx([In, Out] MEMORYSTATUSEX lpBuffer);

        private SystemSpecs _specs;
        private List<TweakItem> _allTweaks = new List<TweakItem>();
        private List<BloatPackage> _allBloat = new List<BloatPackage>();
        private Dictionary<string, PresetConfig> _presets = new Dictionary<string, PresetConfig>(StringComparer.OrdinalIgnoreCase);
        private DispatcherTimer _telemetryTimer;
        private PerformanceCounter _cpuCounter;
        private Button[] _navButtons;
        private FrameworkElement[] _navIndicators;
        private bool _isScanning = false;
        private List<CleanerTargetCategory> _cleanerCategories = new List<CleanerTargetCategory>();
        private List<DnsProviderItem> _dnsProviders = new List<DnsProviderItem>();
        private List<SoftwarePackageItem> _softwarePackages = new List<SoftwarePackageItem>();

        public MainWindow()
        {
            InitializeComponent();
            InitializeWindowChrome();
            InitializeNavigation();
            InitializeSpecs();
            InitializeConsoleDrawer();
            InitializeOverviewBentoCards();
            InitializeQuickBoostActions();
            InitializeFreeFireActions();
            InitializeActiveGameModeDaemon();
            InitializeDeepCleaner();
            InitializeNetworkEngine();
            InitializeRuntimeHub();
            InitializeSettingsAndBackups();

            ThemeManager.LoadSavedTheme(this);

            Loaded += MainWindow_Loaded;
        }

        private async void MainWindow_Loaded(object sender, RoutedEventArgs e)
        {
            InitializeTelemetry();

            // Load presets and tweaks asynchronously
            _presets = TweakService.LoadPresets();
            await LoadAndAuditTweaksAsync();
            await SyncEmulatorHudAsync();
            await LoadAndRenderBackupsListAsync();

            // Trim working set after initial UI realization
            var _op = Dispatcher.BeginInvoke(new Action(() =>
            {
                try
                {
                    GC.Collect();
                    GC.WaitForPendingFinalizers();
                    GC.Collect();
                    EmptyWorkingSet(Process.GetCurrentProcess().Handle);
                }
                catch { }
            }), DispatcherPriority.ApplicationIdle);
        }

        #region Window Chrome & TitleBar
        protected override void OnSourceInitialized(EventArgs e)
        {
            base.OnSourceInitialized(e);
            try
            {
                IntPtr handle = new WindowInteropHelper(this).Handle;
                HwndSource source = HwndSource.FromHwnd(handle);
                if (source != null)
                {
                    source.AddHook(WindowProc);
                }

                this.MaxHeight = SystemParameters.WorkArea.Height;
                this.MaxWidth = SystemParameters.WorkArea.Width;
            }
            catch { }
        }

        private IntPtr WindowProc(IntPtr hwnd, int msg, IntPtr wParam, IntPtr lParam, ref bool handled)
        {
            if (msg == 0x0024) // WM_GETMINMAXINFO
            {
                WmGetMinMaxInfo(hwnd, lParam);
                handled = true;
            }
            return IntPtr.Zero;
        }

        private static void WmGetMinMaxInfo(IntPtr hwnd, IntPtr lParam)
        {
            try
            {
                MINMAXINFO mmi = (MINMAXINFO)Marshal.PtrToStructure(lParam, typeof(MINMAXINFO));

                const int MONITOR_DEFAULTTONEAREST = 0x00000002;
                IntPtr monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);

                if (monitor != IntPtr.Zero)
                {
                    MONITORINFO monitorInfo = new MONITORINFO();
                    GetMonitorInfo(monitor, monitorInfo);

                    RECT rcWorkArea = monitorInfo.rcWork;
                    RECT rcMonitorArea = monitorInfo.rcMonitor;

                    mmi.ptMaxPosition.x = Math.Abs(rcWorkArea.left - rcMonitorArea.left);
                    mmi.ptMaxPosition.y = Math.Abs(rcWorkArea.top - rcMonitorArea.top);
                    mmi.ptMaxSize.x = Math.Abs(rcWorkArea.right - rcWorkArea.left);
                    mmi.ptMaxSize.y = Math.Abs(rcWorkArea.bottom - rcWorkArea.top);
                }

                Marshal.StructureToPtr(mmi, lParam, true);
            }
            catch { }
        }

        private void InitializeWindowChrome()
        {
            // Set official window and taskbar icon
            try
            {
                if (this.Icon == null)
                {
                    Uri iconUri = new Uri("pack://application:,,,/Assets/app_icon.png", UriKind.RelativeOrAbsolute);
                    this.Icon = System.Windows.Media.Imaging.BitmapFrame.Create(iconUri);
                }
            }
            catch
            {
                try
                {
                    string localPng = System.IO.Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "Assets", "app_icon.png");
                    if (System.IO.File.Exists(localPng))
                    {
                        this.Icon = System.Windows.Media.Imaging.BitmapFrame.Create(new Uri(localPng));
                    }
                    else
                    {
                        string localIcon = System.IO.Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "Assets", "icon.ico");
                        if (System.IO.File.Exists(localIcon))
                        {
                            var decoder = System.Windows.Media.Imaging.BitmapDecoder.Create(new Uri(localIcon), System.Windows.Media.Imaging.BitmapCreateOptions.None, System.Windows.Media.Imaging.BitmapCacheOption.OnLoad);
                            this.Icon = decoder.Frames.OrderByDescending(f => f.PixelWidth).FirstOrDefault() ?? decoder.Frames[0];
                        }
                    }
                }
                catch { }
            }

            if (BtnMin != null)
                BtnMin.Click += (s, e) => WindowState = WindowState.Minimized;

            if (BtnMax != null)
                BtnMax.Click += (s, e) => WindowState = WindowState == WindowState.Maximized ? WindowState.Normal : WindowState.Maximized;

            if (BtnClose != null)
                BtnClose.Click += (s, e) => Close();

            if (BrdTitleBar != null)
            {
                BrdTitleBar.MouseLeftButtonDown += (s, e) =>
                {
                    if (e.ButtonState == MouseButtonState.Pressed)
                        DragMove();
                };
            }

            // Global search box jumping to tweaks
            if (TxtGlobalSearch != null)
            {
                TxtGlobalSearch.TextChanged += (s, e) =>
                {
                    string q = TxtGlobalSearch.Text.Trim();
                    if (!string.IsNullOrEmpty(q))
                    {
                        SetNavActive(2); // Go to Tweaks tab
                        if (TxtTweakSearch != null)
                            TxtTweakSearch.Text = q;
                    }
                };
            }
        }
        #endregion

        #region Navigation & Active Pills (All 9 Tabs)
        private void InitializeNavigation()
        {
            _navButtons = new Button[]
            {
                NavBtnOverview,    // 0
                NavBtnQuickBoost,   // 1
                NavBtnTweaks,       // 2
                NavBtnDebloat,      // 3
                NavBtnHealth,       // 4
                NavBtnEmulators,    // 5
                NavBtnSettings,     // 6
                NavBtnBackups,      // 7
                NavBtnAbout         // 8
            };

            _navIndicators = new FrameworkElement[]
            {
                NavIndicatorOverview,
                NavIndicatorQuickBoost,
                NavIndicatorTweaks,
                NavIndicatorDebloat,
                NavIndicatorHealth,
                NavIndicatorEmulators,
                null,
                null,
                null
            };

            for (int i = 0; i < _navButtons.Length; i++)
            {
                int index = i;
                if (_navButtons[i] != null)
                {
                    _navButtons[i].Click += (s, e) => SetNavActive(index);
                }
            }

            // Start on Overview Tab
            SetNavActive(0);
        }

        public void SetNavActive(int index)
        {
            if (TabsWorkspace != null && index >= 0 && index < TabsWorkspace.Items.Count)
                TabsWorkspace.SelectedIndex = index;

            if (_navButtons == null) return;

            for (int i = 0; i < _navButtons.Length; i++)
            {
                bool isActive = (i == index);
                if (i < _navIndicators.Length && _navIndicators[i] != null)
                    _navIndicators[i].Visibility = isActive ? Visibility.Visible : Visibility.Collapsed;

                if (_navButtons[i] != null)
                {
                    if (isActive)
                    {
                        _navButtons[i].SetResourceReference(Control.BackgroundProperty, "SidebarActiveBgBrush");
                        _navButtons[i].SetResourceReference(Control.ForegroundProperty, "SidebarActiveFgBrush");
                    }
                    else
                    {
                        _navButtons[i].Background = Brushes.Transparent;
                        _navButtons[i].SetResourceReference(Control.ForegroundProperty, "TextSecondaryBrush");
                    }
                }
            }

            if (index == 7)
            {
                Dispatcher.BeginInvoke(new Action(async () => await LoadAndRenderBackupsListAsync()));
            }
        }
        #endregion

        #region Single OLED Black Theme
        // Pure OLED Black theme permanently active - zero theme switching overhead
        #endregion

        #region Telemetry & System Specs
        private void InitializeSpecs()
        {
            _specs = HardwareService.GetHardwareSpecs();

            if (TxtHardwareCPU != null) TxtHardwareCPU.Text = _specs.CpuName;
            if (TxtHardwareRAM != null) TxtHardwareRAM.Text = _specs.RamSummary;
            if (TxtHardwareGPU != null) TxtHardwareGPU.Text = _specs.GpuName;
            if (TxtHardwareStorage != null) TxtHardwareStorage.Text = _specs.StorageSummary;
            if (TxtHardwareOS != null) TxtHardwareOS.Text = _specs.OsName;
            if (TxtEditionBadge != null) TxtEditionBadge.Text = _specs.EditionBadge;

            if (TxtGreetingTime != null)
            {
                int hour = DateTime.Now.Hour;
                string greeting = (hour >= 5 && hour < 12) ? "Good Morning" 
                                : (hour >= 12 && hour < 17) ? "Good Afternoon" 
                                : (hour >= 17 && hour < 22) ? "Good Evening" 
                                : "Good Night";

                string rawUser = Environment.UserName;
                string cleanUser = "User";
                if (!string.IsNullOrWhiteSpace(rawUser))
                {
                    cleanUser = rawUser.Trim();
                    if (cleanUser.Contains("\\"))
                    {
                        cleanUser = cleanUser.Substring(cleanUser.LastIndexOf('\\') + 1);
                    }
                    if (cleanUser.Length > 1 && (cleanUser == cleanUser.ToLowerInvariant() || cleanUser == cleanUser.ToUpperInvariant()))
                    {
                        cleanUser = char.ToUpperInvariant(cleanUser[0]) + cleanUser.Substring(1).ToLowerInvariant();
                    }
                }

                TxtGreetingTime.Text = greeting + ", " + cleanUser + "!";
            }
        }

        private static Geometry CreateArcGeometry(double pct, double cx = 18, double cy = 18, double r = 15)
        {
            if (pct <= 0.0) return null;
            double p = Math.Max(0.5, Math.Min(99.9, pct));
            double angleRad = (p / 100.0) * 360.0 * Math.PI / 180.0;
            double x = Math.Round(cx + r * Math.Sin(angleRad), 2);
            double y = Math.Round(cy - r * Math.Cos(angleRad), 2);
            string isLarge = p > 50.0 ? "1" : "0";
            double topY = cy - r;
            string pathData = string.Format(System.Globalization.CultureInfo.InvariantCulture, "M {0},{1} A {2},{2} 0 {3},1 {4},{5}", cx, topY, r, isLarge, x, y);
            try
            {
                return Geometry.Parse(pathData);
            }
            catch
            {
                return null;
            }
        }

        private void InitializeTelemetry()
        {
            try
            {
                _cpuCounter = new PerformanceCounter("Processor", "% Processor Time", "_Total");
                _cpuCounter.NextValue();
            }
            catch { }

            _telemetryTimer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(1.5) };
            _telemetryTimer.Tick += (s, e) =>
            {
                try
                {
                    // 1. CPU Gauge
                    if (_cpuCounter != null && TxtGaugeCPU != null)
                    {
                        int cpuVal = Math.Max(1, (int)_cpuCounter.NextValue());
                        TxtGaugeCPU.Text = cpuVal.ToString() + "%";
                        if (ArcGaugeCPU != null)
                        {
                            ArcGaugeCPU.Data = CreateArcGeometry(cpuVal);
                        }
                    }

                    // 2. RAM Gauge - Real System Memory Utilization % & Live Used / Installed GB
                    if (TxtGaugeRAM != null)
                    {
                        var mem = new MEMORYSTATUSEX();
                        if (GlobalMemoryStatusEx(mem))
                        {
                            int ramPct = (int)mem.dwMemoryLoad;
                            double totalGb = mem.ullTotalPhys / (1024.0 * 1024.0 * 1024.0);
                            double availGb = mem.ullAvailPhys / (1024.0 * 1024.0 * 1024.0);
                            double usedGb = Math.Max(0, totalGb - availGb);
                            double installedGb = Math.Ceiling(totalGb);

                            TxtGaugeRAM.Text = ramPct.ToString() + "%";
                            if (TxtGaugeRAMSub != null)
                            {
                                TxtGaugeRAMSub.Text = string.Format("{0:0.#} / {1:0} GB", usedGb, installedGb);
                            }
                            if (ArcGaugeRAM != null)
                            {
                                ArcGaugeRAM.Data = CreateArcGeometry(ramPct);
                            }

                            TxtGaugeRAM.ToolTip = string.Format(
                                "Live System RAM: {0:0.#} GB Used / {1:0} GB Installed ({2:0.#} GB Usable)\nAvailable Free: {3:0.#} GB ({4}% Load)",
                                usedGb, installedGb, totalGb, availGb, ramPct);
                        }
                    }

                    // 3. SSD / Storage Gauge (C:)
                    if (TxtGaugeDisk != null)
                    {
                        var driveC = new DriveInfo("C");
                        if (driveC.IsReady)
                        {
                            double totalGb = driveC.TotalSize / (1024.0 * 1024.0 * 1024.0);
                            double freeGb = driveC.AvailableFreeSpace / (1024.0 * 1024.0 * 1024.0);
                            double usedGb = Math.Max(0, totalGb - freeGb);
                            int usedPct = totalGb > 0 ? (int)Math.Round((usedGb / totalGb) * 100.0) : 0;

                            TxtGaugeDisk.Text = usedPct.ToString() + "%";
                            if (TxtGaugeDiskSub != null)
                            {
                                TxtGaugeDiskSub.Text = string.Format("{0:0} GB Free", freeGb);
                            }
                            if (ArcGaugeDisk != null)
                            {
                                ArcGaugeDisk.Data = CreateArcGeometry(Math.Max(2, usedPct));
                            }

                            string multiDriveDetails = string.Format(
                                "SSD C: (System): {0:0.#} GB Used / {1:0.#} GB Total ({2:0.#} GB Free, {3}% Used)",
                                usedGb, totalGb, freeGb, usedPct);

                            try
                            {
                                var driveD = new DriveInfo("D");
                                if (driveD.IsReady)
                                {
                                    double dTot = driveD.TotalSize / (1024.0 * 1024.0 * 1024.0);
                                    double dFree = driveD.AvailableFreeSpace / (1024.0 * 1024.0 * 1024.0);
                                    double dUsed = Math.Max(0, dTot - dFree);
                                    int dPct = (int)Math.Round((dUsed / dTot) * 100.0);
                                    multiDriveDetails += string.Format(
                                        "\nDisk D: (Data): {0:0.#} GB Used / {1:0.#} GB Total ({2:0.#} GB Free, {3}% Used)",
                                        dUsed, dTot, dFree, dPct);
                                }
                            }
                            catch { }

                            TxtGaugeDisk.ToolTip = multiDriveDetails;
                        }
                    }
                }
                catch { }
            };
            _telemetryTimer.Start();
        }
        #endregion

        #region Activity Logs Console Drawer
        private void InitializeConsoleDrawer()
        {
            // The main Activity Logs button in the Overview Dock
            if (BtnOpenActivityLogs != null)
                BtnOpenActivityLogs.Click += (s, e) => ToggleConsoleDrawer();

            // Toggle inside drawer
            if (BtnToggleLogs != null)
                BtnToggleLogs.Click += (s, e) => ToggleConsoleDrawer();

            if (BtnCloseDrawer != null)
                BtnCloseDrawer.Click += (s, e) => SetConsoleDrawer(false);

            if (BtnClearLogs != null)
                BtnClearLogs.Click += (s, e) => { if (TxtConsoleLogs != null) TxtConsoleLogs.Text = ""; };

            if (BtnCopyLogs != null)
                BtnCopyLogs.Click += (s, e) =>
                {
                    if (TxtConsoleLogs != null && !string.IsNullOrEmpty(TxtConsoleLogs.Text))
                    {
                        Clipboard.SetText(TxtConsoleLogs.Text);
                        Log("[INFO] Activity logs copied to clipboard.");
                    }
                };
        }

        public void ToggleConsoleDrawer()
        {
            if (BrdConsoleDrawer != null)
            {
                bool isVisible = BrdConsoleDrawer.Visibility == Visibility.Visible;
                SetConsoleDrawer(!isVisible);
            }
        }

        public void SetConsoleDrawer(bool visible)
        {
            if (BrdConsoleDrawer != null)
                BrdConsoleDrawer.Visibility = visible ? Visibility.Visible : Visibility.Collapsed;
        }

        public void Log(string message)
        {
            Dispatcher.Invoke(() =>
            {
                string timestamp = DateTime.Now.ToString("HH:mm:ss");
                string formatted = string.Format("[{0}] {1}", timestamp, message);

                if (TxtStatusBar != null)
                    TxtStatusBar.Text = message;

                if (TxtConsoleLogs != null)
                {
                    TxtConsoleLogs.AppendText(formatted + Environment.NewLine);
                    TxtConsoleLogs.ScrollToEnd();
                }
            });
        }
        #endregion

        #region Overview 6 Bento Action Cards & Scan
        private void InitializeOverviewBentoCards()
        {
            // "Scan My PC" master trigger
            if (BtnOverviewScan != null)
                BtnOverviewScan.Click += async (s, e) => await RunMasterScanAsync();

            // Card 1: Fix Broken Windows Files (SFC)
            if (BtnOverviewSFC != null)
                BtnOverviewSFC.Click += async (s, e) => { SetConsoleDrawer(true); await MaintenanceService.RunSfcAsync(Log); };
            if (BtnNavCardSFC != null)
                BtnNavCardSFC.Click += (s, e) => SetNavActive(4);

            // Card 2: Deep Windows Repair (DISM)
            if (BtnOverviewDISM != null)
                BtnOverviewDISM.Click += async (s, e) => { SetConsoleDrawer(true); await MaintenanceService.RunDismRestoreHealthAsync(Log); };
            if (BtnNavCardDISM != null)
                BtnNavCardDISM.Click += (s, e) => SetNavActive(4);

            // Card 3: Clean Old Windows Updates (Component Cleanup)
            if (BtnOverviewCleanup != null)
                BtnOverviewCleanup.Click += async (s, e) => { SetConsoleDrawer(true); await MaintenanceService.RunComponentCleanupAsync(Log); };
            if (BtnNavCardCleanup != null)
                BtnNavCardCleanup.Click += (s, e) => SetNavActive(4);

            // Card 4: Free Up Trapped Memory (RAM Flush)
            if (BtnOverviewRAM != null)
                BtnOverviewRAM.Click += async (s, e) => await MaintenanceService.FlushRamAsync(Log);
            if (BtnNavCardRAM != null)
                BtnNavCardRAM.Click += (s, e) => SetNavActive(4);

            // Card 5: Turn Off Sleep File (Hibernation Toggle)
            if (BtnOverviewHiber != null)
                BtnOverviewHiber.Click += async (s, e) => await ToggleHibernationAsync();
            if (BtnNavCardHiber != null)
                BtnNavCardHiber.Click += (s, e) => SetNavActive(4);

            // Card 6: Clean Temp & Game Caches
            if (BtnOverviewCache != null)
                BtnOverviewCache.Click += async (s, e) => await MaintenanceService.CleanCacheAsync(Log);
            if (BtnNavCardCache != null)
                BtnNavCardCache.Click += (s, e) => SetNavActive(4);
        }

        private async Task RunMasterScanAsync()
        {
            if (_isScanning) return;
            _isScanning = true;

            if (TxtOverviewScan != null) TxtOverviewScan.Text = "Scanning...";
            Log("[SCAN] Starting comprehensive system health and optimization audit...");

            await LoadAndAuditTweaksAsync();
            await SyncEmulatorHudAsync();
            await LoadAndRenderBackupsListAsync();

            if (TxtOverviewScan != null) TxtOverviewScan.Text = "Scan My PC";
            if (TxtSystemStatusSub != null)
                TxtSystemStatusSub.Text = string.Format("Last scan: Today, {0:hh:mm tt}", DateTime.Now);

            Log("[SUCCESS] System scan complete. All system states audited.");
            _isScanning = false;
        }

        private async Task ToggleHibernationAsync()
        {
            await Task.Run(() =>
            {
                try
                {
                    Log("Toggling Windows Hibernation file (hiberfil.sys)...");
                    bool exists = File.Exists(@"C:\hiberfil.sys");
                    string arg = exists ? "/hibernate off" : "/hibernate on";
                    using (var p = Process.Start(new ProcessStartInfo
                    {
                        FileName = "powercfg.exe",
                        Arguments = arg,
                        CreateNoWindow = true,
                        UseShellExecute = false
                    }))
                    {
                        p.WaitForExit(3000);
                    }
                    if (exists)
                        Log("Hibernation turned OFF. Freed system drive storage.");
                    else
                        Log("Hibernation turned ON.");
                }
                catch (Exception ex)
                {
                    Log("Hibernation error: " + ex.Message);
                }
            });
        }
        #endregion

        #region Quick Boost Tab Actions
        private void InitializeQuickBoostActions()
        {
            // Presets
            if (BtnApplyPresetSafe != null)
                BtnApplyPresetSafe.Click += async (s, e) => await ApplyPresetAsync("safe");
            if (BtnApplyPresetGaming != null)
                BtnApplyPresetGaming.Click += async (s, e) => await ApplyPresetAsync("gaming");
            if (BtnApplyPresetEmulator != null)
                BtnApplyPresetEmulator.Click += async (s, e) => await ApplyPresetAsync("emulator");

            // Master Free Fire hyper boost
            if (BtnMasterFreeFire != null)
                BtnMasterFreeFire.Click += async (s, e) => { await EmulatorService.ApplyMasterFreeFireBoostAsync(Log); await SyncEmulatorHudAsync(); };

            // Revert all
            if (BtnRevertAll != null)
                BtnRevertAll.Click += async (s, e) => await RevertAllTweaksAsync();

            // Standalone quick maintenance buttons
            if (BtnFlushRAM != null)
                BtnFlushRAM.Click += async (s, e) => await MaintenanceService.FlushRamAsync(Log);
            if (BtnToggleHibernation != null)
                BtnToggleHibernation.Click += async (s, e) => await ToggleHibernationAsync();
            if (BtnCleanCaches != null)
                BtnCleanCaches.Click += async (s, e) => await MaintenanceService.CleanCacheAsync(Log);

            if (BtnRunSFC != null)
                BtnRunSFC.Click += async (s, e) => { SetConsoleDrawer(true); await MaintenanceService.RunSfcAsync(Log); };
            if (BtnRunDISM != null)
                BtnRunDISM.Click += async (s, e) => { SetConsoleDrawer(true); await MaintenanceService.RunDismRestoreHealthAsync(Log); };
            if (BtnRunComponentCleanup != null)
                BtnRunComponentCleanup.Click += async (s, e) => { SetConsoleDrawer(true); await MaintenanceService.RunComponentCleanupAsync(Log); };

            // Scan PC State button on Quick Boost tab
            if (BtnScanSystem != null)
                BtnScanSystem.Click += async (s, e) => await RunMasterScanAsync();
        }
        #endregion

        #region Emulators & Free Fire Actions
        private void InitializeFreeFireActions()
        {
            if (BtnFFUnlockFPS != null)
                BtnFFUnlockFPS.Click += async (s, e) => { await EmulatorService.UnlockFpsAsync(Log); await SyncEmulatorHudAsync(); };
            if (BtnFFGpuLock != null)
                BtnFFGpuLock.Click += async (s, e) => { await EmulatorService.LockGpuAsync(Log); await SyncEmulatorHudAsync(); };
            if (BtnFFMouseAim != null)
                BtnFFMouseAim.Click += async (s, e) => { await EmulatorService.ApplySmoothAimAsync(Log); await SyncEmulatorHudAsync(); };
            if (BtnFFLowPing != null)
                BtnFFLowPing.Click += async (s, e) => { await EmulatorService.ApplyLowPingAsync(Log); await SyncEmulatorHudAsync(); };
            if (BtnFFCleanCache != null)
                BtnFFCleanCache.Click += async (s, e) => { await EmulatorService.CleanEmulatorCacheAsync(Log); await SyncEmulatorHudAsync(); };
            if (BtnFFRevert != null)
                BtnFFRevert.Click += async (s, e) => { await EmulatorService.RevertEmulatorSettingsAsync(Log); await SyncEmulatorHudAsync(); };

            if (BtnToggleVBS != null)
                BtnToggleVBS.Click += async (s, e) => { await EmulatorService.ToggleVbsAsync(Log); await SyncEmulatorHudAsync(); };
        }

        #region Active Game Mode Daemon Actions (Feature 2)
        private void InitializeActiveGameModeDaemon()
        {
            if (BtnToggleGameMode != null)
            {
                BtnToggleGameMode.Click += (s, e) =>
                {
                    if (ActiveGameModeDaemon.IsEnabled)
                    {
                        ActiveGameModeDaemon.StopDaemon();
                        BtnToggleGameMode.Content = "START GAME MODE";
                        BtnToggleGameMode.SetResourceReference(Button.StyleProperty, "ButtonPrimary");
                        if (TxtGameModeBadge != null)
                        {
                            TxtGameModeBadge.Text = "DAEMON STOPPED";
                            TxtGameModeBadge.Foreground = new SolidColorBrush(Color.FromRgb(113, 113, 122));
                        }
                        if (BadgeGameModeState != null)
                        {
                            BadgeGameModeState.Background = new SolidColorBrush(Color.FromRgb(18, 18, 22));
                            BadgeGameModeState.BorderBrush = new SolidColorBrush(Color.FromRgb(34, 34, 42));
                        }
                        if (TxtGameModeTarget != null)
                        {
                            TxtGameModeTarget.Text = "Daemon inactive";
                            TxtGameModeTarget.SetResourceReference(TextBlock.ForegroundProperty, "TextMutedBrush");
                        }
                    }
                    else
                    {
                        ActiveGameModeDaemon.EnablePCorePinning = ChkGameModePCores == null || ChkGameModePCores.IsChecked == true;
                        ActiveGameModeDaemon.EnableHighPriority = ChkGameModePriority == null || ChkGameModePriority.IsChecked == true;
                        ActiveGameModeDaemon.Enable1msTimerResolution = ChkGameMode1msTimer == null || ChkGameMode1msTimer.IsChecked == true;
                        ActiveGameModeDaemon.EnablePowerSchemeSwitch = ChkGameModePowerPlan == null || ChkGameModePowerPlan.IsChecked == true;
                        ActiveGameModeDaemon.EnableBackgroundThrottling = ChkGameModeThrottleBg != null && ChkGameModeThrottleBg.IsChecked == true;

                        ActiveGameModeDaemon.StartDaemon();
                        BtnToggleGameMode.Content = "STOP GAME MODE";
                        BtnToggleGameMode.SetResourceReference(Button.StyleProperty, "ButtonSecondary");
                        if (TxtGameModeBadge != null)
                        {
                            TxtGameModeBadge.Text = "MONITORING ACTIVE";
                            TxtGameModeBadge.Foreground = new SolidColorBrush(Color.FromRgb(0, 245, 212));
                        }
                        if (BadgeGameModeState != null)
                        {
                            BadgeGameModeState.Background = new SolidColorBrush(Color.FromRgb(12, 29, 27));
                            BadgeGameModeState.BorderBrush = new SolidColorBrush(Color.FromRgb(0, 245, 212));
                        }
                        if (TxtGameModeTarget != null)
                        {
                            TxtGameModeTarget.Text = "Monitoring games & emulators...";
                            TxtGameModeTarget.SetResourceReference(TextBlock.ForegroundProperty, "TextPrimaryBrush");
                        }
                    }
                };
            }

            // Sync toggle options
            if (ChkGameModePCores != null)
                ChkGameModePCores.Click += (s, e) => ActiveGameModeDaemon.EnablePCorePinning = ChkGameModePCores.IsChecked == true;
            if (ChkGameModePriority != null)
                ChkGameModePriority.Click += (s, e) => ActiveGameModeDaemon.EnableHighPriority = ChkGameModePriority.IsChecked == true;
            if (ChkGameMode1msTimer != null)
                ChkGameMode1msTimer.Click += (s, e) => ActiveGameModeDaemon.Enable1msTimerResolution = ChkGameMode1msTimer.IsChecked == true;
            if (ChkGameModePowerPlan != null)
                ChkGameModePowerPlan.Click += (s, e) => ActiveGameModeDaemon.EnablePowerSchemeSwitch = ChkGameModePowerPlan.IsChecked == true;
            if (ChkGameModeThrottleBg != null)
                ChkGameModeThrottleBg.Click += (s, e) => ActiveGameModeDaemon.EnableBackgroundThrottling = ChkGameModeThrottleBg.IsChecked == true;

            // Custom game addition
            if (BtnAddCustomGame != null && TxtCustomGameExe != null)
            {
                BtnAddCustomGame.Click += (s, e) =>
                {
                    string target = TxtCustomGameExe.Text.Trim();
                    if (!string.IsNullOrEmpty(target) && !target.StartsWith("Add custom", StringComparison.OrdinalIgnoreCase))
                    {
                        ActiveGameModeDaemon.AddMonitoredProcess(target);
                        Log(string.Format("Custom target registered in Game Mode: {0}", target));
                        TxtCustomGameExe.Text = string.Empty;
                        MessageBox.Show(string.Format("'{0}' is now monitored by Active Game Mode Daemon.\nOptimizations will automatically apply upon launch.", target), "Target Added", MessageBoxButton.OK, MessageBoxImage.Information);
                    }
                };

                TxtCustomGameExe.GotFocus += (s, e) =>
                {
                    if (TxtCustomGameExe.Text.StartsWith("Add custom", StringComparison.OrdinalIgnoreCase))
                    {
                        TxtCustomGameExe.Text = string.Empty;
                        TxtCustomGameExe.SetResourceReference(TextBox.ForegroundProperty, "TextPrimaryBrush");
                    }
                };
            }

            // Subscribe to daemon status and log events
            ActiveGameModeDaemon.StatusChanged += (args) =>
            {
                Dispatcher.Invoke(() =>
                {
                    if (args.IsGameActive)
                    {
                        if (TxtGameModeTarget != null)
                        {
                            TxtGameModeTarget.Text = string.Format("{0} (PID {1})", args.GameName, args.ProcessId);
                            TxtGameModeTarget.Foreground = Brushes.LimeGreen;
                        }
                        if (TxtGameModeInterventions != null)
                        {
                            TxtGameModeInterventions.Text = args.Summary;
                            TxtGameModeInterventions.Foreground = new SolidColorBrush(Color.FromRgb(0, 245, 212));
                        }
                        if (TxtGameModeStats != null)
                        {
                            TxtGameModeStats.Text = "Timer: 1.0ms (Ultra-Low Latency)";
                            TxtGameModeStats.Foreground = Brushes.LimeGreen;
                        }
                        if (BadgeGameModeState != null)
                        {
                            BadgeGameModeState.Background = new SolidColorBrush(Color.FromRgb(6, 26, 20));
                            BadgeGameModeState.BorderBrush = new SolidColorBrush(Color.FromRgb(16, 185, 129));
                        }
                        if (TxtGameModeBadge != null)
                        {
                            TxtGameModeBadge.Text = "GAME ACTIVE";
                            TxtGameModeBadge.Foreground = new SolidColorBrush(Color.FromRgb(16, 185, 129));
                        }
                    }
                    else
                    {
                        if (TxtGameModeTarget != null)
                        {
                            TxtGameModeTarget.Text = ActiveGameModeDaemon.IsEnabled ? "Monitoring games & emulators..." : "No active game detected";
                            TxtGameModeTarget.SetResourceReference(TextBlock.ForegroundProperty, "TextPrimaryBrush");
                        }
                        if (TxtGameModeInterventions != null)
                        {
                            TxtGameModeInterventions.Text = "P-Core Pinning, 1ms Timer, High Priority, Ultimate Power";
                            TxtGameModeInterventions.Foreground = new SolidColorBrush(Color.FromRgb(56, 189, 248));
                        }
                        if (TxtGameModeStats != null)
                        {
                            TxtGameModeStats.Text = "Timer: 15.6ms (Standard)";
                            TxtGameModeStats.Foreground = new SolidColorBrush(Color.FromRgb(245, 158, 11));
                        }
                        if (BadgeGameModeState != null)
                        {
                            BadgeGameModeState.Background = new SolidColorBrush(Color.FromRgb(18, 18, 22));
                            BadgeGameModeState.BorderBrush = new SolidColorBrush(Color.FromRgb(34, 34, 42));
                        }
                        if (TxtGameModeBadge != null)
                        {
                            TxtGameModeBadge.Text = ActiveGameModeDaemon.IsEnabled ? "DAEMON STANDBY" : "DAEMON STOPPED";
                            TxtGameModeBadge.Foreground = ActiveGameModeDaemon.IsEnabled ? new SolidColorBrush(Color.FromRgb(0, 245, 212)) : new SolidColorBrush(Color.FromRgb(113, 113, 122));
                        }
                    }
                });
            };

            ActiveGameModeDaemon.LogMessage += (msg) => Log(msg);

            // Clean shutdown on window exit
            Closing += (s, e) =>
            {
                if (ActiveGameModeDaemon.IsEnabled)
                {
                    ActiveGameModeDaemon.StopDaemon();
                }
            };
        }
        #endregion

        #region Deep System Cleaner & Shader Cache Purger (Feature 3)
        private void InitializeDeepCleaner()
        {
            _cleanerCategories = DeepCleanerService.GetDefaultCategories();
            RenderCleanerCategories();

            if (BtnScanJunk != null)
                BtnScanJunk.Click += async (s, e) => await ScanJunkAsync();

            if (BtnPurgeJunk != null)
                BtnPurgeJunk.Click += async (s, e) => await PurgeJunkAsync();

            if (BtnSelectAllCleaner != null)
                BtnSelectAllCleaner.Click += (s, e) => SetAllCleanerSelected(true);

            if (BtnDeselectAllCleaner != null)
                BtnDeselectAllCleaner.Click += (s, e) => SetAllCleanerSelected(false);
        }

        private void SetAllCleanerSelected(bool selected)
        {
            if (_cleanerCategories == null) return;
            foreach (var cat in _cleanerCategories)
            {
                cat.IsSelected = selected;
            }
            RenderCleanerCategories();
        }

        private void RenderCleanerCategories()
        {
            if (PnlCleanerCategories == null || _cleanerCategories == null) return;

            PnlCleanerCategories.Children.Clear();

            foreach (var cat in _cleanerCategories)
            {
                var border = new Border
                {
                    CornerRadius = new CornerRadius(8),
                    BorderThickness = new Thickness(1),
                    Margin = new Thickness(0, 0, 0, 7),
                    Padding = new Thickness(12, 9, 12, 9)
                };
                border.SetResourceReference(Border.BackgroundProperty, "TweakCardBg");
                border.SetResourceReference(Border.BorderBrushProperty, "TweakCardBorder");

                var grid = new Grid();
                grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
                grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });

                var leftStack = new StackPanel { VerticalAlignment = VerticalAlignment.Center };

                var chk = new CheckBox
                {
                    IsChecked = cat.IsSelected,
                    Cursor = Cursors.Hand
                };

                var chkContent = new StackPanel { Orientation = Orientation.Horizontal };
                var txtTitle = new TextBlock
                {
                    Text = cat.Title,
                    FontSize = 12.5,
                    FontWeight = FontWeights.Bold
                };
                txtTitle.SetResourceReference(TextBlock.ForegroundProperty, "TextPrimaryBrush");
                chkContent.Children.Add(txtTitle);

                if (cat.FileCount > 0)
                {
                    var txtFiles = new TextBlock
                    {
                        Text = string.Format(" ({0} files)", cat.FileCount),
                        FontSize = 11,
                        Foreground = new SolidColorBrush(Color.FromRgb(148, 163, 184)),
                        Margin = new Thickness(6, 1, 0, 0)
                    };
                    chkContent.Children.Add(txtFiles);
                }

                chk.Content = chkContent;
                chk.Checked += (s, e) => cat.IsSelected = true;
                chk.Unchecked += (s, e) => cat.IsSelected = false;

                leftStack.Children.Add(chk);

                var txtDesc = new TextBlock
                {
                    Text = cat.Description,
                    FontSize = 10.5,
                    Margin = new Thickness(24, 3, 0, 0),
                    TextWrapping = TextWrapping.Wrap
                };
                txtDesc.SetResourceReference(TextBlock.ForegroundProperty, "TextSecondaryBrush");
                leftStack.Children.Add(txtDesc);

                Grid.SetColumn(leftStack, 0);
                grid.Children.Add(leftStack);

                // Right column badge showing size
                var sizeBadge = new Border
                {
                    CornerRadius = new CornerRadius(6),
                    BorderThickness = new Thickness(1),
                    Padding = new Thickness(10, 4, 10, 4),
                    VerticalAlignment = VerticalAlignment.Center,
                    Background = new SolidColorBrush(Color.FromRgb(24, 19, 11)),
                    BorderBrush = new SolidColorBrush(Color.FromRgb(69, 39, 10))
                };
                var txtSize = new TextBlock
                {
                    Text = cat.SizeFormatted,
                    FontSize = 11,
                    FontWeight = FontWeights.Bold,
                    Foreground = new SolidColorBrush(Color.FromRgb(245, 158, 11))
                };
                sizeBadge.Child = txtSize;

                Grid.SetColumn(sizeBadge, 1);
                grid.Children.Add(sizeBadge);

                border.Child = grid;
                PnlCleanerCategories.Children.Add(border);
            }
        }

        private async Task ScanJunkAsync()
        {
            if (BtnScanJunk != null)
            {
                BtnScanJunk.IsEnabled = false;
                BtnScanJunk.Content = "Scanning...";
            }
            if (TxtCleanerStatus != null)
                TxtCleanerStatus.Text = "Scanning all shader, update, and temp locations...";
            if (TxtCleanerStatusBadge != null)
                TxtCleanerStatusBadge.Text = "SCANNING...";

            try
            {
                _cleanerCategories = await DeepCleanerService.ScanAllCategoriesAsync(Log);
                RenderCleanerCategories();

                long totalBytes = _cleanerCategories.Sum(c => c.TotalSizeBytes);
                int totalFiles = _cleanerCategories.Sum(c => c.FileCount);

                string formattedSize;
                if (totalBytes < 1024 * 1024 * 1024)
                    formattedSize = string.Format("{0:0.#} MB", (double)totalBytes / (1024 * 1024));
                else
                    formattedSize = string.Format("{0:0.##} GB", (double)totalBytes / (1024 * 1024 * 1024));

                if (TxtCleanerScanned != null)
                    TxtCleanerScanned.Text = string.Format("{0} ({1} files)", formattedSize, totalFiles);

                if (TxtCleanerStatus != null)
                    TxtCleanerStatus.Text = string.Format("Found {0} junk across {1} files ready to purge.", formattedSize, totalFiles);

                if (TxtCleanerStatusBadge != null)
                {
                    TxtCleanerStatusBadge.Text = "SCAN COMPLETE";
                    TxtCleanerStatusBadge.Foreground = new SolidColorBrush(Color.FromRgb(16, 185, 129));
                }
            }
            catch (Exception ex)
            {
                Log("Junk scan error: " + ex.Message);
            }
            finally
            {
                if (BtnScanJunk != null)
                {
                    BtnScanJunk.IsEnabled = true;
                    BtnScanJunk.Content = "SCAN JUNK";
                }
            }
        }

        private async Task PurgeJunkAsync()
        {
            var selected = _cleanerCategories.Where(c => c.IsSelected).ToList();
            if (selected.Count == 0)
            {
                MessageBox.Show("Please select at least one cache category to purge.", "No Selection", MessageBoxButton.OK, MessageBoxImage.Information);
                return;
            }

            var confirm = MessageBox.Show(
                string.Format("Are you sure you want to purge {0} selected cache categories?\n\nThis will safely clean DirectX/GPU shader caches, Delivery Optimization chunks, browser bytecode caches, and temporary files.", selected.Count),
                "Confirm Deep Cache Purge",
                MessageBoxButton.YesNo,
                MessageBoxImage.Question);

            if (confirm != MessageBoxResult.Yes) return;

            if (BtnPurgeJunk != null)
            {
                BtnPurgeJunk.IsEnabled = false;
                BtnPurgeJunk.Content = "Purging...";
            }
            if (TxtCleanerStatus != null)
                TxtCleanerStatus.Text = "Purging selected caches and flushing DNS...";
            if (TxtCleanerStatusBadge != null)
            {
                TxtCleanerStatusBadge.Text = "PURGING...";
                TxtCleanerStatusBadge.Foreground = new SolidColorBrush(Color.FromRgb(245, 158, 11));
            }

            try
            {
                long freedBytes = await DeepCleanerService.PurgeCategoriesAsync(_cleanerCategories, Log);

                string freedFormatted;
                if (freedBytes < 1024 * 1024 * 1024)
                    freedFormatted = string.Format("{0:0.#} MB", (double)freedBytes / (1024 * 1024));
                else
                    freedFormatted = string.Format("{0:0.##} GB", (double)freedBytes / (1024 * 1024 * 1024));

                if (TxtCleanerFreed != null)
                    TxtCleanerFreed.Text = string.Format("{0} Recovered", freedFormatted);

                if (TxtCleanerStatus != null)
                    TxtCleanerStatus.Text = string.Format("Successfully purged {0} of storage junk!", freedFormatted);

                if (TxtCleanerStatusBadge != null)
                {
                    TxtCleanerStatusBadge.Text = "PURGE COMPLETE";
                    TxtCleanerStatusBadge.Foreground = new SolidColorBrush(Color.FromRgb(16, 185, 129));
                }

                MessageBox.Show(
                    string.Format("Deep Cache Purge Complete!\n\nReclaimed: {0} disk storage.\nFlushed Windows DNS resolver cache.\nDirectX and GPU shader pipelines cleared for smooth gameplay.", freedFormatted),
                    "Storage Junk Purged",
                    MessageBoxButton.OK,
                    MessageBoxImage.Information);

                // Rescan remaining
                await ScanJunkAsync();
            }
            catch (Exception ex)
            {
                Log("Purge error: " + ex.Message);
            }
            finally
            {
                if (BtnPurgeJunk != null)
                {
                    BtnPurgeJunk.IsEnabled = true;
                    BtnPurgeJunk.Content = "PURGE SELECTED";
                }
            }
        }
        #endregion

        #region Low-Latency Network & Multi-Threaded DNS Engine (Feature 4)
        private void InitializeNetworkEngine()
        {
            if (TxtActiveAdapterName != null)
                TxtActiveAdapterName.Text = NetworkService.GetActiveNetworkInterfaceName();

            _dnsProviders = NetworkService.GetDefaultProviders();
            RenderDnsProviders();

            if (BtnBenchmarkDns != null)
                BtnBenchmarkDns.Click += async (s, e) => await BenchmarkDnsAsync();

            if (BtnApplyFastestDns != null)
                BtnApplyFastestDns.Click += async (s, e) => await ApplyFastestDnsAsync();

            if (BtnResetDns != null)
                BtnResetDns.Click += async (s, e) => await ResetDnsAsync();

            if (BtnOptimizeTcpStack != null)
                BtnOptimizeTcpStack.Click += async (s, e) => await OptimizeTcpStackAsync();
        }

        private void RenderDnsProviders()
        {
            if (PnlDnsProviders == null || _dnsProviders == null) return;

            PnlDnsProviders.Children.Clear();

            foreach (var provider in _dnsProviders)
            {
                var border = new Border
                {
                    CornerRadius = new CornerRadius(8),
                    BorderThickness = new Thickness(1),
                    Margin = new Thickness(0, 0, 0, 7),
                    Padding = new Thickness(12, 9, 12, 9)
                };
                border.SetResourceReference(Border.BackgroundProperty, "TweakCardBg");
                border.SetResourceReference(Border.BorderBrushProperty, "TweakCardBorder");

                var grid = new Grid();
                grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
                grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
                grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });

                // Left Column: Name & Description
                var leftStack = new StackPanel { VerticalAlignment = VerticalAlignment.Center };
                var titleStack = new StackPanel { Orientation = Orientation.Horizontal };

                var txtTitle = new TextBlock
                {
                    Text = provider.Name,
                    FontSize = 12.5,
                    FontWeight = FontWeights.Bold
                };
                txtTitle.SetResourceReference(TextBlock.ForegroundProperty, "TextPrimaryBrush");
                titleStack.Children.Add(txtTitle);

                var txtIps = new TextBlock
                {
                    Text = string.Format(" ({0}, {1})", provider.PrimaryDns, provider.SecondaryDns),
                    FontSize = 11,
                    Foreground = new SolidColorBrush(Color.FromRgb(148, 163, 184)),
                    Margin = new Thickness(6, 1, 0, 0)
                };
                titleStack.Children.Add(txtIps);

                if (provider.IsFastest)
                {
                    var fastBadge = new Border
                    {
                        CornerRadius = new CornerRadius(4),
                        Background = new SolidColorBrush(Color.FromRgb(6, 38, 24)),
                        BorderBrush = new SolidColorBrush(Color.FromRgb(16, 185, 129)),
                        BorderThickness = new Thickness(1),
                        Padding = new Thickness(6, 1, 6, 1),
                        Margin = new Thickness(8, 0, 0, 0),
                        VerticalAlignment = VerticalAlignment.Center
                    };
                    var txtFast = new TextBlock
                    {
                        Text = "FASTEST",
                        FontSize = 9,
                        FontWeight = FontWeights.Bold,
                        Foreground = new SolidColorBrush(Color.FromRgb(16, 185, 129))
                    };
                    fastBadge.Child = txtFast;
                    titleStack.Children.Add(fastBadge);
                }

                leftStack.Children.Add(titleStack);

                var txtDesc = new TextBlock
                {
                    Text = provider.Description,
                    FontSize = 10.5,
                    Margin = new Thickness(0, 3, 0, 0),
                    TextWrapping = TextWrapping.Wrap
                };
                txtDesc.SetResourceReference(TextBlock.ForegroundProperty, "TextSecondaryBrush");
                leftStack.Children.Add(txtDesc);

                Grid.SetColumn(leftStack, 0);
                grid.Children.Add(leftStack);

                // Middle Column: Latency Badge
                var latencyBadge = new Border
                {
                    CornerRadius = new CornerRadius(6),
                    BorderThickness = new Thickness(1),
                    Padding = new Thickness(10, 4, 10, 4),
                    Margin = new Thickness(10, 0, 10, 0),
                    VerticalAlignment = VerticalAlignment.Center
                };

                var txtLatency = new TextBlock
                {
                    Text = provider.LatencyDisplay,
                    FontSize = 11,
                    FontWeight = FontWeights.Bold
                };

                if (provider.IsFastest)
                {
                    latencyBadge.Background = new SolidColorBrush(Color.FromRgb(6, 38, 24));
                    latencyBadge.BorderBrush = new SolidColorBrush(Color.FromRgb(16, 185, 129));
                    txtLatency.Foreground = new SolidColorBrush(Color.FromRgb(16, 185, 129));
                }
                else if (provider.PingMs > 0)
                {
                    latencyBadge.Background = new SolidColorBrush(Color.FromRgb(18, 18, 24));
                    latencyBadge.BorderBrush = new SolidColorBrush(Color.FromRgb(42, 42, 56));
                    txtLatency.Foreground = new SolidColorBrush(Color.FromRgb(56, 189, 248));
                }
                else
                {
                    latencyBadge.Background = new SolidColorBrush(Color.FromRgb(18, 18, 24));
                    latencyBadge.BorderBrush = new SolidColorBrush(Color.FromRgb(42, 42, 56));
                    txtLatency.Foreground = new SolidColorBrush(Color.FromRgb(113, 113, 122));
                }

                latencyBadge.Child = txtLatency;
                Grid.SetColumn(latencyBadge, 1);
                grid.Children.Add(latencyBadge);

                // Right Column: Apply 1-Click Button
                var btnApplyThis = new Button
                {
                    Content = "Apply",
                    FontSize = 10.5,
                    Padding = new Thickness(12, 4, 12, 4),
                    VerticalAlignment = VerticalAlignment.Center
                };
                btnApplyThis.SetResourceReference(Button.StyleProperty, "ButtonSecondary");
                var currentProvider = provider;
                btnApplyThis.Click += async (s, e) =>
                {
                    bool ok = await NetworkService.ApplyDnsToActiveAdapterAsync(currentProvider, Log);
                    if (ok)
                    {
                        MessageBox.Show(
                            string.Format("{0} applied to active network card '{1}'.\n\nWindows DNS cache successfully flushed.", currentProvider.Name, NetworkService.GetActiveNetworkInterfaceName()),
                            "DNS Applied",
                            MessageBoxButton.OK,
                            MessageBoxImage.Information);
                    }
                };

                Grid.SetColumn(btnApplyThis, 2);
                grid.Children.Add(btnApplyThis);

                border.Child = grid;
                PnlDnsProviders.Children.Add(border);
            }
        }

        private async Task BenchmarkDnsAsync()
        {
            if (BtnBenchmarkDns != null)
            {
                BtnBenchmarkDns.IsEnabled = false;
                BtnBenchmarkDns.Content = "Pinging...";
            }
            if (TxtDnsBenchmarkStatus != null)
                TxtDnsBenchmarkStatus.Text = "Sending ICMP packets across all providers concurrently...";
            if (TxtNetworkStatusBadge != null)
                TxtNetworkStatusBadge.Text = "BENCHMARKING...";

            try
            {
                _dnsProviders = await NetworkService.BenchmarkAllProvidersAsync(Log);
                RenderDnsProviders();

                var fastest = _dnsProviders.FirstOrDefault(p => p.IsFastest);
                if (fastest != null)
                {
                    if (TxtDnsBenchmarkStatus != null)
                        TxtDnsBenchmarkStatus.Text = string.Format("Fastest: {0} ({1} ms round-trip time)", fastest.Name, fastest.PingMs);

                    if (TxtNetworkStatusBadge != null)
                    {
                        TxtNetworkStatusBadge.Text = string.Format("FASTEST: {0}ms", fastest.PingMs);
                        TxtNetworkStatusBadge.Foreground = new SolidColorBrush(Color.FromRgb(16, 185, 129));
                    }
                }
            }
            catch (Exception ex)
            {
                Log("DNS benchmark error: " + ex.Message);
            }
            finally
            {
                if (BtnBenchmarkDns != null)
                {
                    BtnBenchmarkDns.IsEnabled = true;
                    BtnBenchmarkDns.Content = "BENCHMARK DNS";
                }
            }
        }

        private async Task ApplyFastestDnsAsync()
        {
            var fastest = _dnsProviders.FirstOrDefault(p => p.IsFastest);
            if (fastest == null || fastest.PingMs <= 0)
            {
                await BenchmarkDnsAsync();
                fastest = _dnsProviders.FirstOrDefault(p => p.IsFastest);
            }

            if (fastest == null)
            {
                MessageBox.Show("Could not find a responding DNS provider. Check your network connection.", "DNS Benchmark Error", MessageBoxButton.OK, MessageBoxImage.Warning);
                return;
            }

            bool ok = await NetworkService.ApplyDnsToActiveAdapterAsync(fastest, Log);
            if (ok)
            {
                MessageBox.Show(
                    string.Format("Successfully applied the fastest DNS ({0}) to active adapter '{1}'.\n\nRound-trip latency: {2} ms\nWindows DNS cache flushed.", fastest.Name, NetworkService.GetActiveNetworkInterfaceName(), fastest.PingMs),
                    "Fastest DNS Applied",
                    MessageBoxButton.OK,
                    MessageBoxImage.Information);
            }
        }

        private async Task ResetDnsAsync()
        {
            bool ok = await NetworkService.ResetDnsToAutomaticAsync(Log);
            if (ok)
            {
                MessageBox.Show(
                    string.Format("DNS configuration on '{0}' has been restored to Automatic (DHCP).\n\nWindows DNS cache flushed.", NetworkService.GetActiveNetworkInterfaceName()),
                    "DNS Restored to Automatic",
                    MessageBoxButton.OK,
                    MessageBoxImage.Information);
            }
        }

        private async Task OptimizeTcpStackAsync()
        {
            bool ok = await NetworkService.ApplyLowLatencyTcpStackAsync(Log);
            if (ok)
            {
                MessageBox.Show(
                    "Competitive Low-Latency TCP Stack Optimized!\n\n" +
                    "- Nagle's Algorithm Disabled (TcpAckFrequency=1, TCPNoDelay=1)\n" +
                    "- Multimedia Throttling Index Disabled\n" +
                    "- SystemResponsiveness Set to 0 (Zero Packet Buffering)\n" +
                    "- Explicit Congestion Notification (ECN) Enabled\n\n" +
                    "Game packets will now be transmitted immediately without artificial delay.",
                    "TCP/IP Stack Tuned",
                    MessageBoxButton.OK,
                    MessageBoxImage.Information);
            }
        }
        #endregion

        #region Automated Gaming Runtimes & WinGet Hub (Feature 5)
        private void InitializeRuntimeHub()
        {
            bool hasWinGet = RuntimeHubService.IsWinGetAvailable();
            if (TxtWinGetStatus != null)
            {
                TxtWinGetStatus.Text = hasWinGet ? "WinGet CLI Active (Silent Mode)" : "Direct Microsoft CDN Mode";
                TxtWinGetStatus.Foreground = hasWinGet ? new SolidColorBrush(Color.FromRgb(16, 185, 129)) : new SolidColorBrush(Color.FromRgb(245, 158, 11));
            }

            _softwarePackages = RuntimeHubService.GetCatalog();
            RenderSoftwarePackages();

            if (BtnScanSoftware != null)
                BtnScanSoftware.Click += async (s, e) => await ScanSoftwareAsync();

            if (BtnInstallSelectedSoftware != null)
                BtnInstallSelectedSoftware.Click += async (s, e) => await InstallSelectedSoftwareAsync();

            if (BtnSelectAllSoftware != null)
                BtnSelectAllSoftware.Click += (s, e) => SelectMissingSoftware(true);

            if (BtnDeselectAllSoftware != null)
                BtnDeselectAllSoftware.Click += (s, e) => SelectMissingSoftware(false);
        }

        private void SelectMissingSoftware(bool selectOnlyMissing)
        {
            if (_softwarePackages == null) return;
            foreach (var pkg in _softwarePackages)
            {
                pkg.IsSelected = selectOnlyMissing ? !pkg.IsInstalled : false;
            }
            RenderSoftwarePackages();
        }

        private void RenderSoftwarePackages()
        {
            if (PnlSoftwarePackages == null || _softwarePackages == null) return;

            PnlSoftwarePackages.Children.Clear();

            foreach (var pkg in _softwarePackages)
            {
                var border = new Border
                {
                    CornerRadius = new CornerRadius(8),
                    BorderThickness = new Thickness(1),
                    Margin = new Thickness(0, 0, 0, 7),
                    Padding = new Thickness(12, 9, 12, 9)
                };
                border.SetResourceReference(Border.BackgroundProperty, "TweakCardBg");
                border.SetResourceReference(Border.BorderBrushProperty, "TweakCardBorder");

                var grid = new Grid();
                grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
                grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
                grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });

                // Left Column: CheckBox, Name, Category pill, Description
                var leftStack = new StackPanel { VerticalAlignment = VerticalAlignment.Center };
                var titleStack = new StackPanel { Orientation = Orientation.Horizontal };

                var chk = new CheckBox
                {
                    IsChecked = pkg.IsSelected,
                    Cursor = Cursors.Hand,
                    VerticalAlignment = VerticalAlignment.Center
                };
                chk.Checked += (s, e) => pkg.IsSelected = true;
                chk.Unchecked += (s, e) => pkg.IsSelected = false;

                var txtTitle = new TextBlock
                {
                    Text = pkg.Name,
                    FontSize = 12.5,
                    FontWeight = FontWeights.Bold,
                    Margin = new Thickness(6, 0, 0, 0),
                    VerticalAlignment = VerticalAlignment.Center
                };
                txtTitle.SetResourceReference(TextBlock.ForegroundProperty, "TextPrimaryBrush");

                var catPill = new Border
                {
                    CornerRadius = new CornerRadius(4),
                    Background = new SolidColorBrush(Color.FromRgb(29, 14, 46)),
                    BorderBrush = new SolidColorBrush(Color.FromRgb(88, 28, 135)),
                    BorderThickness = new Thickness(1),
                    Padding = new Thickness(6, 1, 6, 1),
                    Margin = new Thickness(8, 0, 0, 0),
                    VerticalAlignment = VerticalAlignment.Center
                };
                var txtCat = new TextBlock
                {
                    Text = pkg.Category,
                    FontSize = 9,
                    FontWeight = FontWeights.Bold,
                    Foreground = new SolidColorBrush(Color.FromRgb(168, 85, 247))
                };
                catPill.Child = txtCat;

                titleStack.Children.Add(chk);
                titleStack.Children.Add(txtTitle);
                titleStack.Children.Add(catPill);
                leftStack.Children.Add(titleStack);

                var txtDesc = new TextBlock
                {
                    Text = pkg.Description,
                    FontSize = 10.5,
                    Margin = new Thickness(24, 3, 0, 0),
                    TextWrapping = TextWrapping.Wrap
                };
                txtDesc.SetResourceReference(TextBlock.ForegroundProperty, "TextSecondaryBrush");
                leftStack.Children.Add(txtDesc);

                Grid.SetColumn(leftStack, 0);
                grid.Children.Add(leftStack);

                // Middle Column: Status Badge
                var statusBadge = new Border
                {
                    CornerRadius = new CornerRadius(6),
                    BorderThickness = new Thickness(1),
                    Padding = new Thickness(10, 4, 10, 4),
                    Margin = new Thickness(10, 0, 10, 0),
                    VerticalAlignment = VerticalAlignment.Center
                };
                var txtStatus = new TextBlock
                {
                    Text = pkg.IsInstalled ? "INSTALLED" : "AVAILABLE",
                    FontSize = 10,
                    FontWeight = FontWeights.Bold
                };

                if (pkg.IsInstalled)
                {
                    statusBadge.Background = new SolidColorBrush(Color.FromRgb(6, 38, 24));
                    statusBadge.BorderBrush = new SolidColorBrush(Color.FromRgb(16, 185, 129));
                    txtStatus.Foreground = new SolidColorBrush(Color.FromRgb(16, 185, 129));
                }
                else
                {
                    statusBadge.Background = new SolidColorBrush(Color.FromRgb(38, 26, 12));
                    statusBadge.BorderBrush = new SolidColorBrush(Color.FromRgb(245, 158, 11));
                    txtStatus.Foreground = new SolidColorBrush(Color.FromRgb(245, 158, 11));
                }
                statusBadge.Child = txtStatus;

                Grid.SetColumn(statusBadge, 1);
                grid.Children.Add(statusBadge);

                // Right Column: Individual Install Button
                var btnInstallThis = new Button
                {
                    Content = pkg.IsInstalled ? "Reinstall" : "Install",
                    FontSize = 10.5,
                    Padding = new Thickness(12, 4, 12, 4),
                    VerticalAlignment = VerticalAlignment.Center
                };
                btnInstallThis.SetResourceReference(Button.StyleProperty, "ButtonSecondary");
                var currentPkg = pkg;
                btnInstallThis.Click += async (s, e) =>
                {
                    btnInstallThis.IsEnabled = false;
                    btnInstallThis.Content = "Installing...";
                    bool ok = await RuntimeHubService.InstallPackageAsync(currentPkg, Log);
                    btnInstallThis.IsEnabled = true;
                    btnInstallThis.Content = ok ? "Reinstall" : "Retry";
                    await ScanSoftwareAsync();
                };

                Grid.SetColumn(btnInstallThis, 2);
                grid.Children.Add(btnInstallThis);

                border.Child = grid;
                PnlSoftwarePackages.Children.Add(border);
            }
        }

        private async Task ScanSoftwareAsync()
        {
            if (BtnScanSoftware != null)
            {
                BtnScanSoftware.IsEnabled = false;
                BtnScanSoftware.Content = "Checking...";
            }
            if (TxtSoftwareStatus != null)
                TxtSoftwareStatus.Text = "Scanning Windows registry and program paths for installed runtimes...";
            if (TxtSoftwareStatusBadge != null)
                TxtSoftwareStatusBadge.Text = "SCANNING...";

            try
            {
                await RuntimeHubService.CheckInstalledStatusAsync(_softwarePackages, Log);
                RenderSoftwarePackages();

                int installed = _softwarePackages.Count(p => p.IsInstalled);
                int missing = _softwarePackages.Count - installed;

                if (TxtSoftwareStatus != null)
                    TxtSoftwareStatus.Text = string.Format("Found {0} installed, {1} ready to deploy.", installed, missing);

                if (TxtSoftwareStatusBadge != null)
                {
                    TxtSoftwareStatusBadge.Text = string.Format("{0} MISSING", missing);
                    TxtSoftwareStatusBadge.Foreground = missing == 0 ? new SolidColorBrush(Color.FromRgb(16, 185, 129)) : new SolidColorBrush(Color.FromRgb(245, 158, 11));
                }
            }
            catch (Exception ex)
            {
                Log("Software check error: " + ex.Message);
            }
            finally
            {
                if (BtnScanSoftware != null)
                {
                    BtnScanSoftware.IsEnabled = true;
                    BtnScanSoftware.Content = "CHECK RUNTIMES";
                }
            }
        }

        private async Task InstallSelectedSoftwareAsync()
        {
            var selected = _softwarePackages.Where(p => p.IsSelected && !p.IsInstalled).ToList();
            if (selected.Count == 0)
            {
                MessageBox.Show("Please select at least one missing software package or runtime to install.", "No Packages Selected", MessageBoxButton.OK, MessageBoxImage.Information);
                return;
            }

            var confirm = MessageBox.Show(
                string.Format("Are you sure you want to install {0} selected software package(s)?\n\nPackages will be downloaded directly from official Microsoft/vendor endpoints and installed silently in the background.", selected.Count),
                "Confirm Silent Software Installation",
                MessageBoxButton.YesNo,
                MessageBoxImage.Question);

            if (confirm != MessageBoxResult.Yes) return;

            if (BtnInstallSelectedSoftware != null)
            {
                BtnInstallSelectedSoftware.IsEnabled = false;
                BtnInstallSelectedSoftware.Content = "Installing...";
            }
            if (TxtSoftwareStatus != null)
                TxtSoftwareStatus.Text = string.Format("Deploying {0} packages silently in background...", selected.Count);
            if (TxtSoftwareStatusBadge != null)
            {
                TxtSoftwareStatusBadge.Text = "INSTALLING...";
                TxtSoftwareStatusBadge.Foreground = new SolidColorBrush(Color.FromRgb(168, 85, 247));
            }

            try
            {
                int success = await RuntimeHubService.InstallSelectedBatchAsync(_softwarePackages, Log);

                MessageBox.Show(
                    string.Format("Deployment Complete!\n\nSuccessfully installed: {0} of {1} package(s).\n\nYour gaming runtimes are now up to date.", success, selected.Count),
                    "Software Deployment Hub",
                    MessageBoxButton.OK,
                    MessageBoxImage.Information);

                await ScanSoftwareAsync();
            }
            catch (Exception ex)
            {
                Log("Installation error: " + ex.Message);
            }
            finally
            {
                if (BtnInstallSelectedSoftware != null)
                {
                    BtnInstallSelectedSoftware.IsEnabled = true;
                    BtnInstallSelectedSoftware.Content = "INSTALL SELECTED";
                }
            }
        }
        #endregion

        private async Task SyncEmulatorHudAsync()
        {
            await Task.Run(() =>
            {
                var topo = EmulatorService.GetTopology();
                Dispatcher.Invoke(() =>
                {
                    if (TxtBS5Status != null)
                    {
                        TxtBS5Status.Text = topo.BlueStacksInstalled ? ("Installed (" + topo.BlueStacksVersion + ")") : "Not Detected";
                        TxtBS5Status.Foreground = topo.BlueStacksInstalled ? System.Windows.Media.Brushes.LimeGreen : System.Windows.Media.Brushes.SlateGray;
                    }
                    if (TxtMSIStatus != null)
                    {
                        TxtMSIStatus.Text = topo.MsiInstalled ? ("Installed (" + topo.MsiVersion + ")") : "Not Detected";
                        TxtMSIStatus.Foreground = topo.MsiInstalled ? System.Windows.Media.Brushes.LimeGreen : System.Windows.Media.Brushes.SlateGray;
                    }
                    if (TxtROGStatus != null)
                    {
                        TxtROGStatus.Text = topo.FpsUnlocked ? "Unlocked (240 FPS)" : (topo.RogProfileActive ? "ROG 2 Profile" : "Standard 60 FPS");
                        TxtROGStatus.Foreground = topo.FpsUnlocked ? System.Windows.Media.Brushes.Cyan : (topo.RogProfileActive ? System.Windows.Media.Brushes.LimeGreen : System.Windows.Media.Brushes.SlateGray);
                    }
                    if (TxtMouseAimStatus != null)
                    {
                        TxtMouseAimStatus.Text = topo.RawMouseActive ? "1:1 Raw (Linear)" : "Standard Curve";
                        TxtMouseAimStatus.Foreground = topo.RawMouseActive ? System.Windows.Media.Brushes.LimeGreen : System.Windows.Media.Brushes.SlateGray;
                    }
                    if (TxtVBSStatus != null)
                    {
                        TxtVBSStatus.Text = topo.VbsStatusSummary;
                        TxtVBSStatus.Foreground = topo.VbsStatusSummary.IndexOf("Off", StringComparison.OrdinalIgnoreCase) >= 0 ? System.Windows.Media.Brushes.LimeGreen : System.Windows.Media.Brushes.Orange;
                    }
                });
            });
        }
        #endregion

        #region Settings & Backups Tabs
        private void InitializeSettingsAndBackups()
        {
            if (RdoOSWin11 != null)
                RdoOSWin11.Checked += async (s, e) => { _specs.IsWin11 = true; await LoadAndAuditTweaksAsync(); Log("Target OS set to Windows 11."); };
            if (RdoOSWin10 != null)
                RdoOSWin10.Checked += async (s, e) => { _specs.IsWin11 = false; await LoadAndAuditTweaksAsync(); Log("Target OS set to Windows 10."); };
            if (RdoOSAuto != null)
                RdoOSAuto.Checked += async (s, e) => { _specs = HardwareService.GetHardwareSpecs(); await LoadAndAuditTweaksAsync(); Log("Target OS set to Auto-Detect."); };

            if (BtnResetSettings != null)
            {
                BtnResetSettings.Click += (s, e) =>
                {
                    if (RdoOSAuto != null) RdoOSAuto.IsChecked = true;
                    if (TxtTweakSearch != null) TxtTweakSearch.Text = "";
                    if (TxtGlobalSearch != null) TxtGlobalSearch.Text = "";
                    if (ChkAutoRestorePoint != null) ChkAutoRestorePoint.IsChecked = true;
                    if (_allTweaks != null)
                    {
                        foreach (var t in _allTweaks) t.IsSelected = false;
                    }
                    ThemeManager.ApplyOledTheme(this);
                    Log("[SUCCESS] System preferences, tweak selections, and search filters reset to defaults.");
                };
            }

            if (BtnCreateRestorePoint != null)
                BtnCreateRestorePoint.Click += async (s, e) =>
                {
                    try
                    {
                        BtnCreateRestorePoint.IsEnabled = false;
                        Log("[BACKUP] Creating manual system safety restore point and registry snapshot...");
                        await SafetyService.CreateRestorePointAsync("Manual Safety Checkpoint", Log);
                        await LoadAndRenderBackupsListAsync();
                    }
                    finally
                    {
                        BtnCreateRestorePoint.IsEnabled = true;
                    }
                };

            if (BtnRefreshBackups != null)
                BtnRefreshBackups.Click += async (s, e) => await LoadAndRenderBackupsListAsync();

            if (BtnOpenBackupsFolder != null)
                BtnOpenBackupsFolder.Click += (s, e) => SafetyService.OpenBackupsFolder();

            if (BtnLaunchSystemRestore != null)
                BtnLaunchSystemRestore.Click += (s, e) => SafetyService.LaunchSystemRestoreWizard(Log);

            if (BtnUndoAllRestore != null)
                BtnUndoAllRestore.Click += async (s, e) =>
                {
                    var res = MessageBox.Show(
                        "Are you sure you want to revert every applied tweak and optimization back to Windows defaults?",
                        "Emergency Factory Reset",
                        MessageBoxButton.YesNo,
                        MessageBoxImage.Warning);
                    if (res == MessageBoxResult.Yes)
                    {
                        await RevertAllTweaksAsync();
                    }
                };

            // Debloat
            if (BtnScanBloat != null)
                BtnScanBloat.Click += async (s, e) => await ScanBloatAsync();
            if (BtnRemoveBloat != null)
                BtnRemoveBloat.Click += async (s, e) => await RemoveBloatAsync();

            // Tweak Search
            if (TxtTweakSearch != null)
            {
                TxtTweakSearch.TextChanged += (s, e) =>
                {
                    string q = TxtTweakSearch.Text.Trim();
                    if (string.IsNullOrEmpty(q))
                    {
                        RenderTweakCards(_allTweaks);
                    }
                    else
                    {
                        var filtered = _allTweaks.Where(t =>
                            (t.title != null && t.title.IndexOf(q, StringComparison.OrdinalIgnoreCase) >= 0) ||
                            (t.category != null && t.category.IndexOf(q, StringComparison.OrdinalIgnoreCase) >= 0) ||
                            (t.description != null && t.description.IndexOf(q, StringComparison.OrdinalIgnoreCase) >= 0));
                        RenderTweakCards(filtered);
                    }
                };
            }

            // Select / Deselect All Tweaks
            if (BtnSelectAllTweaks != null)
                BtnSelectAllTweaks.Click += (s, e) => { foreach (var t in _allTweaks) t.IsSelected = true; };
            if (BtnDeselectAllTweaks != null)
                BtnDeselectAllTweaks.Click += (s, e) => { foreach (var t in _allTweaks) t.IsSelected = false; };

            // Apply / Revert Selected Tweaks
            if (BtnApplySelectedTweaks != null)
                BtnApplySelectedTweaks.Click += async (s, e) => await ApplySelectedTweaksAsync(true);
            if (BtnRevertSelectedTweaks != null)
                BtnRevertSelectedTweaks.Click += async (s, e) => await ApplySelectedTweaksAsync(false);
        }
        #endregion

        #region Tweaks Engine
        private async Task LoadAndAuditTweaksAsync()
        {
            _allTweaks = TweakService.LoadTweaks(_specs != null ? _specs.IsWin11 : true);
            RenderTweakCards(_allTweaks);

            // Background audit
            await TweakService.AuditTweaksAsync(_allTweaks, (id, isApplied) =>
            {
                Dispatcher.Invoke(() =>
                {
                    var tw = _allTweaks.FirstOrDefault(t => t.id == id);
                    if (tw != null)
                    {
                        tw.IsApplied = isApplied;
                    }
                });
            }, Log);

            UpdateReadinessScore();
        }

        private void RenderTweakCards(IEnumerable<TweakItem> tweaks)
        {
            if (PnlTweaksContainer == null) return;
            PnlTweaksContainer.Children.Clear();

            var groups = tweaks.GroupBy(t => t.category ?? "General");

            foreach (var g in groups)
            {
                // Category Header
                var header = new TextBlock
                {
                    Text = g.Key.ToUpper(),
                    FontSize = 11,
                    FontWeight = FontWeights.Bold,
                    Margin = new Thickness(2, 14, 0, 6)
                };
                header.SetResourceReference(TextBlock.ForegroundProperty, "AccentBrush");
                PnlTweaksContainer.Children.Add(header);

                foreach (var tw in g)
                {
                    var card = CreateTweakCard(tw);
                    PnlTweaksContainer.Children.Add(card);
                }
            }
        }

        private Border CreateTweakCard(TweakItem tweak)
        {
            var card = new Border
            {
                BorderThickness = new Thickness(1),
                CornerRadius = new CornerRadius(8),
                Margin = new Thickness(0, 0, 0, 7),
                Padding = new Thickness(14, 11, 14, 11)
            };
            card.SetResourceReference(Border.BackgroundProperty, "TweakCardBg");
            card.SetResourceReference(Border.BorderBrushProperty, "TweakCardBorder");

            var grid = new Grid();
            grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });

            // Checkbox and Title/Description Stack
            var chk = new CheckBox
            {
                VerticalAlignment = VerticalAlignment.Center
            };
            chk.SetBinding(CheckBox.IsCheckedProperty, new System.Windows.Data.Binding("IsSelected")
            {
                Source = tweak,
                Mode = System.Windows.Data.BindingMode.TwoWay
            });

            var spText = new StackPanel { Margin = new Thickness(8, 0, 0, 0) };

            var txtTitle = new TextBlock
            {
                Text = tweak.title,
                FontSize = 13,
                FontWeight = FontWeights.SemiBold
            };
            txtTitle.SetResourceReference(TextBlock.ForegroundProperty, "TextPrimaryBrush");

            var txtDesc = new TextBlock
            {
                Text = tweak.description,
                FontSize = 11,
                TextWrapping = TextWrapping.Wrap,
                Margin = new Thickness(0, 3, 0, 0)
            };
            txtDesc.SetResourceReference(TextBlock.ForegroundProperty, "TextSecondaryBrush");

            spText.Children.Add(txtTitle);
            spText.Children.Add(txtDesc);

            var spLeft = new StackPanel { Orientation = Orientation.Horizontal };
            spLeft.Children.Add(chk);
            spLeft.Children.Add(spText);
            Grid.SetColumn(spLeft, 0);

            // Status Badge
            var badgeBorder = new Border
            {
                CornerRadius = new CornerRadius(4),
                Padding = new Thickness(8, 3, 8, 3),
                VerticalAlignment = VerticalAlignment.Center
            };
            var badgeText = new TextBlock
            {
                Text = tweak.StatusBadgeText,
                FontSize = 10,
                FontWeight = FontWeights.Bold
            };

            Action updateBadgeColors = () =>
            {
                if (tweak.IsApplied)
                {
                    badgeBorder.SetResourceReference(Border.BackgroundProperty, "BadgeOptimizedBg");
                    badgeBorder.SetResourceReference(Border.BorderBrushProperty, "BadgeOptimizedBorder");
                    badgeText.SetResourceReference(TextBlock.ForegroundProperty, "BadgeOptimizedFg");
                    badgeText.Text = "OPTIMIZED";
                }
                else
                {
                    badgeBorder.SetResourceReference(Border.BackgroundProperty, "BadgeStandardBg");
                    badgeBorder.SetResourceReference(Border.BorderBrushProperty, "BadgeStandardBorder");
                    badgeText.SetResourceReference(TextBlock.ForegroundProperty, "BadgeStandardFg");
                    badgeText.Text = "STANDARD";
                }
            };
            updateBadgeColors();

            tweak.PropertyChanged += (s, e) =>
            {
                if (e.PropertyName == "IsApplied")
                    updateBadgeColors();
            };

            badgeBorder.Child = badgeText;
            Grid.SetColumn(badgeBorder, 1);

            grid.Children.Add(spLeft);
            grid.Children.Add(badgeBorder);
            card.Child = grid;

            // Hover effects
            card.MouseEnter += (s, e) => card.SetResourceReference(Border.BackgroundProperty, "TweakCardHoverBg");
            card.MouseLeave += (s, e) => card.SetResourceReference(Border.BackgroundProperty, "TweakCardBg");

            return card;
        }

        private void UpdateReadinessScore()
        {
            if (_allTweaks == null || _allTweaks.Count == 0) return;
            int applied = _allTweaks.Count(t => t.IsApplied);
            int total = _allTweaks.Count;
            int score = (int)((double)applied / total * 100);

            if (TxtSystemStatusSub != null)
                TxtSystemStatusSub.Text = string.Format("{0} of {1} optimizations applied ({2}% system score)", applied, total, score);
        }

        private async Task ApplyPresetAsync(string presetKey)
        {
            string key = presetKey;
            if (!_presets.ContainsKey(key))
            {
                if (string.Equals(key, "safe", StringComparison.OrdinalIgnoreCase) && _presets.ContainsKey("safe_baseline"))
                    key = "safe_baseline";
                else if (string.Equals(key, "gaming", StringComparison.OrdinalIgnoreCase) && _presets.ContainsKey("gaming_rig"))
                    key = "gaming_rig";
                else if (string.Equals(key, "emulator", StringComparison.OrdinalIgnoreCase) && _presets.ContainsKey("emulator_pro"))
                    key = "emulator_pro";
            }

            if (!_presets.ContainsKey(key))
            {
                Log(string.Format("Preset {0} not found.", presetKey));
                return;
            }

            var preset = _presets[key];
            string title = !string.IsNullOrEmpty(preset.Title) ? preset.Title : presetKey;
            Log(string.Format("Activating Profile: {0}...", title));

            if (ChkAutoRestorePoint != null && ChkAutoRestorePoint.IsChecked == true)
            {
                await SafetyService.CreateRestorePointAsync(string.Format("AnxiouslyOptimized {0} Snapshot", title), Log);
            }

            foreach (var tId in preset.TweakIds)
            {
                var tw = _allTweaks.FirstOrDefault(t => string.Equals(t.id, tId, StringComparison.OrdinalIgnoreCase));
                if (tw != null)
                {
                    bool ok = await TweakService.ExecuteTweakActionAsync(tw, true, Log);
                    if (ok) tw.IsApplied = true;
                }
            }

            UpdateReadinessScore();
            Log(string.Format("Profile '{0}' applied successfully.", title));
        }

        private async Task ApplySelectedTweaksAsync(bool apply)
        {
            var selected = _allTweaks.Where(t => t.IsSelected).ToList();
            if (selected.Count == 0)
            {
                Log("No tweaks selected.");
                return;
            }

            string action = apply ? "Applying" : "Reverting";
            Log(string.Format("{0} {1} selected tweaks...", action, selected.Count));

            foreach (var tw in selected)
            {
                bool ok = await TweakService.ExecuteTweakActionAsync(tw, apply, Log);
                if (ok) tw.IsApplied = apply;
            }

            UpdateReadinessScore();
            Log(string.Format("{0} batch completed.", action));
        }

        private async Task RevertAllTweaksAsync()
        {
            var applied = _allTweaks.Where(t => t.IsApplied).ToList();
            Log(string.Format("Reverting all {0} applied optimizations...", applied.Count));

            foreach (var tw in applied)
            {
                bool ok = await TweakService.ExecuteTweakActionAsync(tw, false, Log);
                if (ok) tw.IsApplied = false;
            }

            UpdateReadinessScore();
            Log("All optimizations reverted to default state.");
        }
        #endregion

        #region Debloat Engine
        private async Task ScanBloatAsync()
        {
            _allBloat = await DebloatService.ScanInstalledBloatAsync(Log);
            if (PnlBloatContainer != null)
            {
                PnlBloatContainer.Children.Clear();
                foreach (var pkg in _allBloat)
                {
                    var card = new Border
                    {
                        BorderThickness = new Thickness(1),
                        CornerRadius = new CornerRadius(8),
                        Margin = new Thickness(0, 0, 0, 7),
                        Padding = new Thickness(14, 11, 14, 11)
                    };
                    card.SetResourceReference(Border.BackgroundProperty, "TweakCardBg");
                    card.SetResourceReference(Border.BorderBrushProperty, "TweakCardBorder");

                    var sp = new StackPanel();
                    var chk = new CheckBox
                    {
                        Content = pkg.DisplayName + " (" + pkg.PackageName + ")",
                        IsChecked = pkg.IsSelected,
                        FontWeight = FontWeights.SemiBold
                    };
                    chk.SetResourceReference(CheckBox.ForegroundProperty, "TextPrimaryBrush");
                    chk.Checked += (s, e) => pkg.IsSelected = true;
                    chk.Unchecked += (s, e) => pkg.IsSelected = false;

                    var txt = new TextBlock
                    {
                        Text = pkg.Description,
                        FontSize = 11,
                        Margin = new Thickness(24, 3, 0, 0)
                    };
                    txt.SetResourceReference(TextBlock.ForegroundProperty, "TextSecondaryBrush");

                    sp.Children.Add(chk);
                    sp.Children.Add(txt);
                    card.Child = sp;
                    PnlBloatContainer.Children.Add(card);
                }
            }
        }

        private async Task RemoveBloatAsync()
        {
            var selected = _allBloat.Where(b => b.IsSelected).ToList();
            if (selected.Count == 0)
            {
                Log("No bloatware apps selected for removal.");
                return;
            }
            int removed = await DebloatService.RemoveSelectedBloatAsync(_allBloat, Log);
            await ScanBloatAsync();
        }
        #endregion

        #region Backup and Restore System
        private async Task LoadAndRenderBackupsListAsync()
        {
            if (PnlRestorePointsList == null) return;

            try
            {
                if (TxtBackupsCount != null)
                    TxtBackupsCount.Text = "Scanning backup snapshots...";

                var backups = await SafetyService.GetBackupHistoryAsync(Log);

                Dispatcher.Invoke(() =>
                {
                    if (TxtBackupsCount != null)
                    {
                        int count = backups != null ? backups.Count : 0;
                        TxtBackupsCount.Text = string.Format("{0} point{1} recorded", count, count == 1 ? "" : "s");
                    }

                    PnlRestorePointsList.Children.Clear();

                    if (backups == null || backups.Count == 0)
                    {
                        var emptyCard = new Border
                        {
                            CornerRadius = new CornerRadius(8),
                            BorderThickness = new Thickness(1),
                            Padding = new Thickness(16, 14, 16, 14),
                            Margin = new Thickness(0, 0, 0, 6)
                        };
                        emptyCard.SetResourceReference(Border.BackgroundProperty, "TweakCardBg");
                        emptyCard.SetResourceReference(Border.BorderBrushProperty, "TweakCardBorder");

                        var spEmpty = new StackPanel();
                        var t1 = new TextBlock
                        {
                            Text = "No Restore Points or Registry Snapshots Found",
                            FontSize = 12.5,
                            FontWeight = FontWeights.SemiBold
                        };
                        t1.SetResourceReference(TextBlock.ForegroundProperty, "TextPrimaryBrush");

                        var t2 = new TextBlock
                        {
                            Text = "Click 'Create Backup Point' above to generate an immediate Windows System Restore checkpoint and multi-branch registry backup.",
                            FontSize = 11,
                            Margin = new Thickness(0, 4, 0, 0),
                            TextWrapping = TextWrapping.Wrap
                        };
                        t2.SetResourceReference(TextBlock.ForegroundProperty, "TextSecondaryBrush");

                        spEmpty.Children.Add(t1);
                        spEmpty.Children.Add(t2);
                        emptyCard.Child = spEmpty;
                        PnlRestorePointsList.Children.Add(emptyCard);
                        return;
                    }

                    foreach (var b in backups)
                    {
                        var entry = b;
                        var card = new Border
                        {
                            CornerRadius = new CornerRadius(8),
                            BorderThickness = new Thickness(1),
                            Padding = new Thickness(14, 11, 14, 11),
                            Margin = new Thickness(0, 0, 0, 8)
                        };
                        card.SetResourceReference(Border.BackgroundProperty, "TweakCardBg");
                        card.SetResourceReference(Border.BorderBrushProperty, "TweakCardBorder");

                        var grid = new Grid();
                        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
                        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });

                        // Left stack
                        var leftStack = new StackPanel { VerticalAlignment = VerticalAlignment.Center };

                        // Title row with badge
                        var titleRow = new StackPanel { Orientation = Orientation.Horizontal, VerticalAlignment = VerticalAlignment.Center };
                        var txtTitle = new TextBlock
                        {
                            Text = entry.Title,
                            FontSize = 12.5,
                            FontWeight = FontWeights.Bold
                        };
                        txtTitle.SetResourceReference(TextBlock.ForegroundProperty, "TextPrimaryBrush");
                        titleRow.Children.Add(txtTitle);

                        var badge = new Border
                        {
                            CornerRadius = new CornerRadius(4),
                            BorderThickness = new Thickness(1),
                            Padding = new Thickness(6, 2, 6, 2),
                            Margin = new Thickness(8, 0, 0, 0)
                        };
                        var txtBadge = new TextBlock
                        {
                            FontSize = 9,
                            FontWeight = FontWeights.Bold
                        };

                        if (entry.IsSystemRestorePoint)
                        {
                            badge.Background = new SolidColorBrush(Color.FromRgb(15, 41, 34));
                            badge.BorderBrush = new SolidColorBrush(Color.FromRgb(0, 245, 212));
                            txtBadge.Text = "WINDOWS RESTORE POINT";
                            txtBadge.Foreground = new SolidColorBrush(Color.FromRgb(0, 245, 212));
                        }
                        else
                        {
                            badge.Background = new SolidColorBrush(Color.FromRgb(30, 27, 75));
                            badge.BorderBrush = new SolidColorBrush(Color.FromRgb(99, 102, 241));
                            txtBadge.Text = "REGISTRY BACKUP";
                            txtBadge.Foreground = new SolidColorBrush(Color.FromRgb(129, 140, 248));
                        }
                        badge.Child = txtBadge;
                        titleRow.Children.Add(badge);
                        leftStack.Children.Add(titleRow);

                        // Metadata line
                        string meta = entry.TimestampFormatted + " \u2022 " + entry.SizeFormatted;
                        if (!string.IsNullOrEmpty(entry.Description))
                        {
                            meta += " \u2022 " + entry.Description;
                        }
                        var txtMeta = new TextBlock
                        {
                            Text = meta,
                            FontSize = 10.5,
                            Margin = new Thickness(0, 4, 0, 0)
                        };
                        txtMeta.SetResourceReference(TextBlock.ForegroundProperty, "TextMutedBrush");
                        leftStack.Children.Add(txtMeta);

                        // File path if available
                        if (!string.IsNullOrEmpty(entry.FilePath))
                        {
                            var txtPath = new TextBlock
                            {
                                Text = entry.FilePath,
                                FontSize = 9.5,
                                Margin = new Thickness(0, 2, 0, 0),
                                TextTrimming = TextTrimming.CharacterEllipsis
                            };
                            txtPath.SetResourceReference(TextBlock.ForegroundProperty, "TextMutedBrush");
                            leftStack.Children.Add(txtPath);
                        }

                        Grid.SetColumn(leftStack, 0);
                        grid.Children.Add(leftStack);

                        // Right stack of action buttons
                        var rightStack = new StackPanel { Orientation = Orientation.Horizontal, VerticalAlignment = VerticalAlignment.Center };

                        if (entry.IsSystemRestorePoint)
                        {
                            var btnRestore = new Button
                            {
                                Content = "Restore Snapshot",
                                Height = 28,
                                FontSize = 11,
                                Padding = new Thickness(10, 4, 10, 4),
                                Margin = new Thickness(0, 0, 6, 0)
                            };
                            btnRestore.SetResourceReference(Button.StyleProperty, "ButtonPrimary");
                            btnRestore.Click += async (s, e) =>
                            {
                                var res = MessageBox.Show(
                                    string.Format("Restore Windows back to point '{0}' (Sequence #{1})?\n\nThis will reboot your system to restore the Windows snapshot.", entry.Title, entry.SequenceNumber),
                                    "Confirm System Snapshot Restore",
                                    MessageBoxButton.YesNo,
                                    MessageBoxImage.Warning);
                                if (res == MessageBoxResult.Yes)
                                {
                                    await SafetyService.RestoreSystemSnapshotAsync(entry.SequenceNumber, Log);
                                }
                            };
                            rightStack.Children.Add(btnRestore);
                        }
                        else
                        {
                            var btnImport = new Button
                            {
                                Content = "Import Reg",
                                Height = 28,
                                FontSize = 11,
                                Padding = new Thickness(10, 4, 10, 4),
                                Margin = new Thickness(0, 0, 6, 0)
                            };
                            btnImport.SetResourceReference(Button.StyleProperty, "ButtonPrimary");
                            btnImport.Click += async (s, e) =>
                            {
                                var res = MessageBox.Show(
                                    string.Format("Import and restore registry keys from:\n{0}?\n\nThis will re-apply all saved registry settings.", entry.FilePath),
                                    "Confirm Registry Restore",
                                    MessageBoxButton.YesNo,
                                    MessageBoxImage.Question);
                                if (res == MessageBoxResult.Yes)
                                {
                                    bool ok = await SafetyService.ImportRegistryBackupAsync(entry.FilePath, Log);
                                    if (ok)
                                    {
                                        MessageBox.Show("Registry successfully restored from backup file.", "Registry Restore Complete", MessageBoxButton.OK, MessageBoxImage.Information);
                                    }
                                }
                            };
                            rightStack.Children.Add(btnImport);
                        }

                        // Delete button
                        var btnDelete = new Button
                        {
                            Content = "Delete",
                            Height = 28,
                            FontSize = 11,
                            Padding = new Thickness(8, 4, 8, 4)
                        };
                        btnDelete.SetResourceReference(Button.StyleProperty, "ButtonSecondary");
                        btnDelete.Click += async (s, e) =>
                        {
                            var res = MessageBox.Show(
                                string.Format("Delete backup entry '{0}'?", entry.Title),
                                "Confirm Delete",
                                MessageBoxButton.YesNo,
                                MessageBoxImage.Question);
                            if (res == MessageBoxResult.Yes)
                            {
                                SafetyService.DeleteBackupEntry(entry, Log);
                                await LoadAndRenderBackupsListAsync();
                            }
                        };
                        rightStack.Children.Add(btnDelete);

                        Grid.SetColumn(rightStack, 1);
                        grid.Children.Add(rightStack);

                        card.Child = grid;
                        PnlRestorePointsList.Children.Add(card);
                    }
                });
            }
            catch (Exception ex)
            {
                Log("Error rendering backup points: " + ex.Message);
            }
        }
        #endregion
    }
}
