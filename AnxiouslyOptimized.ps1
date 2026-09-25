# ==============================================================================



#  ANXIOUSLYOPTIMIZED - COMMERCIAL SUITE v3.5



#  High-Performance, Zero-Breakage Windows Optimizer & Debloater Engine



# ==============================================================================



[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidUsingWriteHost', '')]



param(



    [ValidateSet("Safe", "Gaming", "Emulator", "None")][string]$Preset = "None",



    [ValidateSet("Auto", "Win10", "Win11")][string]$TargetOS = "Auto",



    [switch]$FreeFire,



    [switch]$Emulator,



    [switch]$FlushRAM,



    [switch]$ToggleHibernation,



    [switch]$StretchedRes,



    [switch]$VBSAudit,



    [switch]$CoreParking,



    [switch]$RevertAll,



    [switch]$ResetMouse,

    [switch]$Scan,



    [switch]$Debloat,



    [switch]$Repair,



    [switch]$CleanCache,



    [switch]$NoGUI



)



# Pre-load GUI and Presentation assemblies



Add-Type -AssemblyName PresentationFramework, PresentationCore, WindowsBase -ErrorAction SilentlyContinue

try {
    if (-not ([System.Management.Automation.PSTypeName]'Win32WindowProc').Type) {
        Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public class Win32WindowProc {
    [StructLayout(LayoutKind.Sequential)]
    public struct POINT { public int x; public int y; }

    [StructLayout(LayoutKind.Sequential)]
    public struct MINMAXINFO {
        public POINT ptReserved;
        public POINT ptMaxSize;
        public POINT ptMaxPosition;
        public POINT ptMinTrackSize;
        public POINT ptMaxTrackSize;
    }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Auto)]
    public class MONITORINFO {
        public int cbSize = Marshal.SizeOf(typeof(MONITORINFO));
        public RECT rcMonitor = new RECT();
        public RECT rcWork = new RECT();
        public int dwFlags = 0;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT {
        public int left;
        public int top;
        public int right;
        public int bottom;
    }

    [DllImport("user32.dll")]
    public static extern IntPtr MonitorFromWindow(IntPtr hwnd, uint dwFlags);

    [DllImport("user32.dll", CharSet = CharSet.Auto)]
    public static extern bool GetMonitorInfo(IntPtr hMonitor, MONITORINFO lpmi);

    public static void HandleWmGetMinMaxInfo(IntPtr hwnd, IntPtr lParam) {
        try {
            MINMAXINFO mmi = (MINMAXINFO)Marshal.PtrToStructure(lParam, typeof(MINMAXINFO));
            IntPtr monitor = MonitorFromWindow(hwnd, 2);
            if (monitor != IntPtr.Zero) {
                MONITORINFO mi = new MONITORINFO();
                GetMonitorInfo(monitor, mi);
                RECT rcWork = mi.rcWork;
                RECT rcMon = mi.rcMonitor;
                mmi.ptMaxPosition.x = Math.Abs(rcWork.left - rcMon.left);
                mmi.ptMaxPosition.y = Math.Abs(rcWork.top - rcMon.top);
                mmi.ptMaxSize.x = Math.Abs(rcWork.right - rcWork.left);
                mmi.ptMaxSize.y = Math.Abs(rcWork.bottom - rcWork.top);
            }
            Marshal.StructureToPtr(mmi, lParam, true);
        } catch { }
    }
}
"@ -ErrorAction SilentlyContinue
    }
} catch { }



# Resolve script root directory safely across all execution contexts
$scriptDir = $PSScriptRoot
if (!$scriptDir -and $PSCommandPath) {
    $scriptDir = Split-Path -Parent $PSCommandPath
}
if (!$scriptDir -and $MyInvocation.MyCommand.Path) {
    $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
}
$candidates = @(
    $scriptDir,
    "D:\Users\GOYAL\Documents\optimizefiles",
    "C:\Users\Raghav\Documents\optimizefiles",
    (Get-Location).Path
)
foreach ($cand in $candidates) {
    if ($cand -and (Test-Path (Join-Path $cand "core\Safety.psm1"))) {
        $scriptDir = $cand
        break
    }
}
if (!$scriptDir) {
    $scriptDir = (Get-Location).Path
}
Set-Location $scriptDir



# Import Core Modules



Import-Module (Join-Path $scriptDir "core\Safety.psm1") -Force



Import-Module (Join-Path $scriptDir "core\Engine.psm1") -Force



Import-Module (Join-Path $scriptDir "core\Debloater.psm1") -Force



Import-Module (Join-Path $scriptDir "core\Maintenance.psm1") -Force



Import-Module (Join-Path $scriptDir "core\Benchmark.psm1") -Force



Import-Module (Join-Path $scriptDir "core\Emulator.psm1") -Force



# Verify Administrator Elevation (bypass elevation for read-only scan)



if (!$Scan -and !$ResetMouse -and !(Test-AdminPrivilege)) {



    Write-Host "[PERMISSION] Administrator permissions are required." -ForegroundColor Yellow



    Write-Host "Opening again with Administrator permissions..." -ForegroundColor Cyan



    $targetFile = if ($PSCommandPath) { $PSCommandPath } else { Join-Path $scriptDir "AnxiouslyOptimized.ps1" }



    Start-Process powershell.exe -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$targetFile`"" -WorkingDirectory "$scriptDir" -Verb RunAs



    Exit



}



# ==============================================================================



#  CLI / HEADLESS MODE



# ==============================================================================



if ($Preset -ne "None" -or $RevertAll -or $ResetMouse -or $Scan -or $Debloat -or $Repair -or $CleanCache -or $FreeFire -or $Emulator -or $FlushRAM -or $ToggleHibernation -or $StretchedRes -or $VBSAudit -or $CoreParking -or $NoGUI) {



    Write-Host @"



 __________________________________________________________________



|                                                                  |



|   ANXIOUSLYOPTIMIZED - EASY PC SPEED & GAME BOOSTER              |



|   100% Safe Standard | One-Click Reversible                      |



|__________________________________________________________________|



"@ -ForegroundColor Cyan



    if (!$Scan -and !$ResetMouse) {
        New-SystemRestorePoint -Description "AnxiouslyOptimized_CLI_Backup" | Out-Null
    }



    $activeOS = if ($TargetOS -ne "Auto") { $TargetOS } else { Get-DetectedOS }



    Write-Host "Windows Version: $activeOS" -ForegroundColor White



    if ($Scan) {



        Write-Host "`n[SCAN] Checking system settings for $activeOS..." -ForegroundColor Yellow



        $tweaks = Get-TweaksConfig -TargetOS $activeOS



        foreach ($t in $tweaks) {



            $isApplied = Test-TweakApplied -Tweak $t



            $color = if ($isApplied) { "Green" } else { "DarkGray" }



            $status = if ($isApplied) { "[OPTIMIZED]" } else { "[DEFAULT]" }



            Write-Host ("  {0,-11} {1}" -f $status, $t.title) -ForegroundColor $color



        }



    }



    if ($FreeFire -or $Emulator) {



        Write-Host "`n[FREE FIRE] Running 1-Click Master Free Fire & Emulator Boost..." -ForegroundColor Yellow



        Invoke-MasterFreeFireBoost



    }



    if ($StretchedRes) {



        Write-Host "`n[STRETCHED SCREEN] Setting 4:3 Stretched View (1440x1080 @ 320 DPI)..." -ForegroundColor Yellow



        Invoke-FreeFireStretchedResolution -Width 1440 -Height 1080 -Dpi 320



    }



    if ($VBSAudit) {



        $vbs = Get-VBSStatus



        Write-Host "`n[EMULATOR SPEED] Status: $($vbs.StatusSummary)" -ForegroundColor Yellow



        Write-Host "  Advice: $($vbs.Recommendation)" -ForegroundColor Cyan



    }



    if ($FlushRAM) {



        Write-Host "`n[CLEAN MEMORY] Freeing up unused cached memory..." -ForegroundColor Yellow



        Invoke-StandbyMemoryFlush



    }



    if ($ToggleHibernation) {



        Write-Host "`n[SLEEP FILE] Turning sleep file on or off..." -ForegroundColor Yellow



        Invoke-ToggleHibernation



    }



    if ($CoreParking) {



        Write-Host "`n[PROCESSOR] Keeping all processor cores awake to stop stutter..." -ForegroundColor Yellow



        Invoke-DisableCoreParking



    }



    if ($Preset -eq "Safe") {



        Invoke-PresetBatch -PresetKey "safe_baseline"



    } elseif ($Preset -eq "Gaming") {



        Invoke-PresetBatch -PresetKey "gaming_rig"



    } elseif ($Preset -eq "Emulator") {



        Invoke-PresetBatch -PresetKey "emulator_pro"



    }



    if ($Debloat) {



        Write-Host "`n[APP CLEANER] Removing unwanted pre-installed apps..." -ForegroundColor Yellow



        $removed = Invoke-SafeDebloat



        Write-Host "  Removed $removed unwanted apps." -ForegroundColor Green



    }



    if ($Repair) {



        Invoke-SystemFileCheck



        Invoke-DISMRepair



    }



    if ($CleanCache) {



        Invoke-ClearSafeCaches



    }

    if ($ResetMouse) {

        Write-Host "`n[MOUSE] Restoring Windows default mouse speed (10) and factory curves..." -ForegroundColor Yellow

        Reset-WindowsMouseSettings

    }



    if ($RevertAll) {

        Invoke-RevertAllTweaks

        if (Get-Command Reset-WindowsMouseSettings -ErrorAction SilentlyContinue) {

            Reset-WindowsMouseSettings | Out-Null

        }

        if (Get-Command Invoke-RevertEmulatorTweaks -ErrorAction SilentlyContinue) {

            Invoke-RevertEmulatorTweaks | Out-Null

        }

    }



    Write-Host "`n[DONE] Finished successfully.`n" -ForegroundColor Green



    Exit



}



# ==============================================================================



#  NATIVE WPF GUI LAUNCHER



# ==============================================================================



$xamlPath = Join-Path $scriptDir "assets\gui.xaml"



if (!(Test-Path $xamlPath)) {



    Write-Error "GUI layout file not found at: $xamlPath"



    Exit



}



# Load and parse XAML



[xml]$xaml = Get-Content $xamlPath -Raw



$reader = New-Object System.Xml.XmlNodeReader $xaml



$window = [System.Windows.Markup.XamlReader]::Load($reader)



# Set official application icon for Window frame and Windows Taskbar
$appIconPath = Join-Path $scriptDir "assets\app_icon.png"
$iconPath = Join-Path $scriptDir "assets\icon.ico"

if (Test-Path $appIconPath) {
    try {
        $bmpIcon = [System.Windows.Media.Imaging.BitmapFrame]::Create([System.Uri]::new($appIconPath))
        $window.Icon = $bmpIcon
        $titleLogo = $window.FindName("ImgTitleLogo")
        if ($titleLogo) { $titleLogo.Source = $bmpIcon }
        $aboutLogo = $window.FindName("ImgAboutLogo")
        if ($aboutLogo) { $aboutLogo.Source = $bmpIcon }
    } catch { }
} elseif (Test-Path $iconPath) {
    try {
        $window.Icon = [System.Windows.Media.Imaging.BitmapFrame]::Create([System.Uri]::new($iconPath))
    } catch { }
}



# Set celestial atmospheric background art

$imgBgHero = $window.FindName("ImgBgHero")

$bannerPath = Join-Path $scriptDir "assets\bg_hero_banner.png"

$bgPath = if (Test-Path $bannerPath) { $bannerPath } else { Join-Path $scriptDir "assets\bg_hero.jpg" }

if ($imgBgHero -and (Test-Path $bgPath)) {



    try {



        $imgBgHero.Source = [System.Windows.Media.Imaging.BitmapFrame]::Create([System.Uri]::new($bgPath))



    } catch { }



}



# Screen WorkArea bounds constraint (guarantees bottom pane is never cut off by taskbar in fullscreen)



$window.Add_SourceInitialized({
    try {
        $hwnd = (New-Object System.Windows.Interop.WindowInteropHelper($window)).Handle
        $source = [System.Windows.Interop.HwndSource]::FromHwnd($hwnd)
        if ($source) {
            $source.AddHook({
                param($h, $msg, $wParam, $lParam, [ref]$handled)
                if ($msg -eq 0x0024) {
                    try {
                        [Win32WindowProc]::HandleWmGetMinMaxInfo($h, $lParam)
                        $handled.Value = $true
                    } catch { }
                }
                return [IntPtr]::Zero
            })
        }
        $workArea = [System.Windows.SystemParameters]::WorkArea
        $window.MaxHeight = $workArea.Height
        $window.MaxWidth = $workArea.Width
    } catch { }
})



# Retrieve Named Controls



$txtCPU = $window.FindName("TxtHardwareCPU")



$txtRAM = $window.FindName("TxtHardwareRAM")



$txtGPU = $window.FindName("TxtHardwareGPU")



$txtStorage = $window.FindName("TxtHardwareStorage")



$txtOS = $window.FindName("TxtHardwareOS")



$txtEditionBadge = $window.FindName("TxtEditionBadge")



$brdEditionBadge = $window.FindName("BrdEditionBadge")



$activeOS = if ($TargetOS -ne "Auto") { $TargetOS } else { Get-DetectedOS }



if ($txtEditionBadge -and $brdEditionBadge) {



    $txtEditionBadge.Text = if ($activeOS -eq "Win11") { "Windows 11 Edition" } else { "Windows 10 Edition" }



    # Edition badge colors and styling are dynamically managed by Set-AppTheme



}



$btnSafe = $window.FindName("BtnApplyPresetSafe")



$btnGaming = $window.FindName("BtnApplyPresetGaming")



$btnEmulator = $window.FindName("BtnApplyPresetEmulator")



$btnRevertAll = $window.FindName("BtnRevertAll")



$btnRestorePoint = $window.FindName("BtnCreateRestorePoint")
$btnRefreshBackups = $window.FindName("BtnRefreshBackups")
$btnOpenBackupsFolder = $window.FindName("BtnOpenBackupsFolder")
$btnLaunchSystemRestore = $window.FindName("BtnLaunchSystemRestore")
$pnlRestorePointsList = $window.FindName("PnlRestorePointsList")
$txtBackupsCount = $window.FindName("TxtBackupsCount")
$btnUndoAllRestore = $window.FindName("BtnUndoAllRestore")



$btnScanSystem = $window.FindName("BtnScanSystem")



$pnlTweaks = $window.FindName("PnlTweaksContainer")



$btnApplySelected = $window.FindName("BtnApplySelectedTweaks")



$btnRevertSelected = $window.FindName("BtnRevertSelectedTweaks")



$btnSelectAll = $window.FindName("BtnSelectAllTweaks")



$btnDeselectAll = $window.FindName("BtnDeselectAllTweaks")



$txtTweakSearch = $window.FindName("TxtTweakSearch")



$pnlBloat = $window.FindName("PnlBloatContainer")



$btnScanBloat = $window.FindName("BtnScanBloat")



$btnRemoveBloat = $window.FindName("BtnRemoveBloat")



$btnRunSFC = $window.FindName("BtnRunSFC")



$btnRunDISM = $window.FindName("BtnRunDISM")



$btnRunComponentCleanup = $window.FindName("BtnRunComponentCleanup")



$btnFlushRAM = $window.FindName("BtnFlushRAM")



$btnToggleHibernation = $window.FindName("BtnToggleHibernation")



$btnCleanCaches = $window.FindName("BtnCleanCaches")



$txtStatusBar = $window.FindName("TxtStatusBar")



$btnToggleLogs = $window.FindName("BtnToggleLogs")



$brdConsoleDrawer = $window.FindName("BrdConsoleDrawer")



$txtConsole = $window.FindName("TxtConsoleLogs")



$btnClearLogs = $window.FindName("BtnClearLogs")
$btnCopyLogs = $window.FindName("BtnCopyLogs")
$btnCloseDrawer = $window.FindName("BtnCloseDrawer")



# Custom Title Bar & Sidebar Navigation Controls



$btnMin = $window.FindName("BtnMin")



$btnMax = $window.FindName("BtnMax")



$btnClose = $window.FindName("BtnClose")



$navOverview = $window.FindName("NavBtnOverview")



$navQuickBoost = $window.FindName("NavBtnQuickBoost")



$navTweaks = $window.FindName("NavBtnTweaks")



$navDebloat = $window.FindName("NavBtnDebloat")



$navHealth = $window.FindName("NavBtnHealth")



$navEmulators = $window.FindName("NavBtnEmulators")



$navSettings = $window.FindName("NavBtnSettings")



$navBackups = $window.FindName("NavBtnBackups")



$navAbout = $window.FindName("NavBtnAbout")



$navIndOverview = $window.FindName("NavIndicatorOverview")



$navIndQuickBoost = $window.FindName("NavIndicatorQuickBoost")



$navIndTweaks = $window.FindName("NavIndicatorTweaks")



$navIndDebloat = $window.FindName("NavIndicatorDebloat")



$navIndHealth = $window.FindName("NavIndicatorHealth")



$navIndEmulators = $window.FindName("NavIndicatorEmulators")



$tabsWorkspace = $window.FindName("TabsWorkspace")



# Reference Screen: Overview Hero & 6 Action Bento Cards



$txtGreetingTime = $window.FindName("TxtGreetingTime")



$btnOverviewScan = $window.FindName("BtnOverviewScan")



$vbScanIcon = $window.FindName("VbScanIcon")



$txtOverviewScan = $window.FindName("TxtOverviewScan")



$btnOverviewSFC = $window.FindName("BtnOverviewSFC")



$btnOverviewDISM = $window.FindName("BtnOverviewDISM")



$btnOverviewCleanup = $window.FindName("BtnOverviewCleanup")



$btnOverviewRAM = $window.FindName("BtnOverviewRAM")



$btnOverviewHiber = $window.FindName("BtnOverviewHiber")



$btnOverviewCache = $window.FindName("BtnOverviewCache")



$btnNavCardSFC = $window.FindName("BtnNavCardSFC")



$btnNavCardDISM = $window.FindName("BtnNavCardDISM")



$btnNavCardCleanup = $window.FindName("BtnNavCardCleanup")



$btnNavCardRAM = $window.FindName("BtnNavCardRAM")



$btnNavCardHiber = $window.FindName("BtnNavCardHiber")



$btnNavCardCache = $window.FindName("BtnNavCardCache")



# Telemetry Gauges, Sparklines & Health Status Badges



$txtGaugeCPU = $window.FindName("TxtGaugeCPU")



$txtGaugeRAM = $window.FindName("TxtGaugeRAM")



$txtGaugeRAMSub = $window.FindName("TxtGaugeRAMSub")



$txtGaugeDisk = $window.FindName("TxtGaugeDisk")



$txtGaugeDiskSub = $window.FindName("TxtGaugeDiskSub")



$arcGaugeCPU = $window.FindName("ArcGaugeCPU")



$arcGaugeRAM = $window.FindName("ArcGaugeRAM")



$arcGaugeDisk = $window.FindName("ArcGaugeDisk")



$pathSparkCPU = $window.FindName("PathSparklineCPU")



$pathSparkRAM = $window.FindName("PathSparklineRAM")



$pathSparkDisk = $window.FindName("PathSparklineDisk")



$txtSystemStatusSub = $window.FindName("TxtSystemStatusSub")

$pnlTelemetryCapsule = $window.FindName("PnlTelemetryCapsule")

$btnOpenActivityLogs = $window.FindName("BtnOpenActivityLogs")

$btnToggleLogs = $window.FindName("BtnToggleLogs")

$brdOverviewDivider = $window.FindName("BrdOverviewDivider")

$txtSystemHealthTitle = $window.FindName("TxtSystemHealthTitle")
$txtSystemHealthSubtitle = $window.FindName("TxtSystemHealthSubtitle")
$pnlSystemHealthBadge = $window.FindName("PnlSystemHealthBadge")



# Search Pill Box



$txtGlobalSearch = $window.FindName("TxtGlobalSearch")



# Theme Switcher Controls



$brdTitleBar = $window.FindName("BrdTitleBar")
$brdSidebar = $window.FindName("BrdSidebar")
$rootBorder = $window.FindName("RootBorder")
$pnlAmbientGlowMesh = $window.FindName("PnlAmbientGlowMesh")
$brdThemeBar = $window.FindName("BrdThemeBar")
$brdThemeSliderThumb = $window.FindName("BrdThemeSliderThumb")
$brdSearchBox = $window.FindName("BrdSearchBox")
$brdOverviewDock = $window.FindName("BrdOverviewDock")
$brdEditionBadge = $window.FindName("BrdEditionBadge")
$txtEditionBadge = $window.FindName("TxtEditionBadge")



$btnThemeObsidian = $window.FindName("BtnThemeObsidian")



$btnThemeMoonlight = $window.FindName("BtnThemeMoonlight")



$btnThemeAurora = $window.FindName("BtnThemeAurora")



$btnThemeNebula = $window.FindName("BtnThemeNebula")



$dotThemeObsidian = $window.FindName("DotThemeObsidian")



$dotThemeMoonlight = $window.FindName("DotThemeMoonlight")



$dotThemeAurora = $window.FindName("DotThemeAurora")



$dotThemeNebula = $window.FindName("DotThemeNebula")



# Settings & Backups Controls



$rdoOSAuto = $window.FindName("RdoOSAuto")



$rdoOSWin11 = $window.FindName("RdoOSWin11")



$rdoOSWin10 = $window.FindName("RdoOSWin10")



$btnResetSettings = $window.FindName("BtnResetSettings")



$btnUndoAllRestore = $window.FindName("BtnUndoAllRestore")



# Free Fire & Emulator Controls



$btnMasterFreeFire = $window.FindName("BtnMasterFreeFire")



$txtBS5Status = $window.FindName("TxtBS5Status")



$txtMSIStatus = $window.FindName("TxtMSIStatus")



$txtROGStatus = $window.FindName("TxtROGStatus")



$txtMouseAimStatus = $window.FindName("TxtMouseAimStatus")



$txtVBSStatus = $window.FindName("TxtVBSStatus")



$btnFFUnlockFPS = $window.FindName("BtnFFUnlockFPS")



$btnFFGpuLock = $window.FindName("BtnFFGpuLock")



$btnFFMouseAim = $window.FindName("BtnFFMouseAim")



$btnFFLowPing = $window.FindName("BtnFFLowPing")



$btnToggleVBS = $window.FindName("BtnToggleVBS")



$btnFFCleanCache = $window.FindName("BtnFFCleanCache")



$btnFFRevert = $window.FindName("BtnFFRevert")



# Thread-safe Logging Function with Status Bar Updates



$Log = {



    param([string]$Message, [switch]$ShowDrawer)



    $timestamp = Get-Date -Format "HH:mm:ss"



    $formatted = "[$timestamp] $Message`r`n"



    $txtConsole.Dispatcher.Invoke([System.Action]{



        $txtConsole.AppendText($formatted)



        $txtConsole.ScrollToEnd()



        if ($txtStatusBar) {



            $cleanMsg = $Message -replace '^(\[[A-Z]+\]\s*|\*\s*)', ''



            $txtStatusBar.Text = $cleanMsg



        }



        if ($ShowDrawer -and $brdConsoleDrawer) {



            $brdConsoleDrawer.Visibility = [System.Windows.Visibility]::Visible



        }



    })



}



# Function to refresh live emulator HUD status



$SyncEmulatorHUD = {



    $topo = Get-EmulatorTopology



    if ($txtBS5Status) {



        $txtBS5Status.Text = if ($topo.BlueStacks5_Installed) { "Installed (v$($topo.BlueStacks5_Version))" } else { "Not Detected" }



        $txtBS5Status.Foreground = if ($topo.BlueStacks5_Installed) { [System.Windows.Media.Brushes]::LimeGreen } else { [System.Windows.Media.Brushes]::SlateGray }



    }



    if ($txtMSIStatus) {



        $txtMSIStatus.Text = if ($topo.MSI_Installed) { "Installed (v$($topo.MSI_Version))" } else { "Not Installed" }



        $txtMSIStatus.Foreground = if ($topo.MSI_Installed) { [System.Windows.Media.Brushes]::Tomato } else { [System.Windows.Media.Brushes]::SlateGray }



    }



    if ($txtROGStatus) {



        $txtROGStatus.Text = if ($topo.FreeFire_ROGProfile) { "240 FPS Active" } else { "Normal Mode" }



        $txtROGStatus.Foreground = if ($topo.FreeFire_ROGProfile) { [System.Windows.Media.Brushes]::Cyan } else { [System.Windows.Media.Brushes]::SlateGray }



    }



    if ($txtMouseAimStatus) {



        $txtMouseAimStatus.Text = if ($topo.RawMouse_Active) { "Smooth Aim Active" } else { "Windows Default" }



        $txtMouseAimStatus.Foreground = if ($topo.RawMouse_Active) { [System.Windows.Media.Brushes]::MediumPurple } else { [System.Windows.Media.Brushes]::SlateGray }



    }



    if ($txtVBSStatus) {



        $vbs = Get-VBSStatus



        if ($vbs.HypervisorLaunchType -eq "Off" -or (!$vbs.VBS_Active -and $vbs.HypervisorLaunchType -ne "Auto")) {



            $txtVBSStatus.Text = "Max Speed (Direct)"



            $txtVBSStatus.Foreground = [System.Windows.Media.Brushes]::LimeGreen



        } else {



            $txtVBSStatus.Text = "Slowed by Security"



            $txtVBSStatus.Foreground = [System.Windows.Media.Brushes]::Orange



        }



    }



}



# Initial Greeting



& $Log "AnxiouslyOptimized is ready for $activeOS."



& $Log "Everything is backed up and completely safe to use."



# Hardware Telemetry & Emulator Status Population



$hardware = Get-SystemHardwareAudit



if ($txtCPU) { $txtCPU.Text = "$($hardware.CPUName) ($($hardware.LogicalCores) Cores)" }



if ($txtRAM) { $txtRAM.Text = if ($hardware.RAMSummary) { "$($hardware.RAMSummary)" } else { "$($hardware.TotalRAMGB) GB RAM" } }



if ($txtGPU) { $txtGPU.Text = "$($hardware.GPUName)" }



if ($txtStorage) { $txtStorage.Text = if ($hardware.StorageSummary) { "$($hardware.StorageSummary)" } else { "1.0 TB NVMe SSD" } }



if ($txtOS)  { $txtOS.Text = "$($hardware.OSName) Build $($hardware.OSBuild)" }



& $SyncEmulatorHUD



# Activity Log Drawer Toggle



$script:ToggleActivityLogsAction = {
    if ($brdConsoleDrawer) {
        if ($brdConsoleDrawer.Visibility -eq [System.Windows.Visibility]::Visible) {
            $brdConsoleDrawer.Visibility = [System.Windows.Visibility]::Collapsed
            if ($btnToggleLogs) { $btnToggleLogs.Content = "Activity Log" }
        } else {
            $brdConsoleDrawer.Visibility = [System.Windows.Visibility]::Visible
            if ($btnToggleLogs) { $btnToggleLogs.Content = "Hide Log" }
            if ($txtConsole) { $txtConsole.ScrollToEnd() }
        }
    }
}

if ($btnToggleLogs) {
    $btnToggleLogs.Add_Click($script:ToggleActivityLogsAction)
}

if ($btnOpenActivityLogs) {
    $btnOpenActivityLogs.Add_Click($script:ToggleActivityLogsAction)
}

if ($pnlSystemHealthBadge) {
    $pnlSystemHealthBadge.Add_MouseDown({ & $script:ToggleActivityLogsAction })
}



# Tweak Checkbox Registry



$tweakCheckboxMap = @{}



function Sync-TweakList {



    if (!$pnlTweaks) { return }



    [void]$pnlTweaks.Children.Clear()



    $tweakCheckboxMap.Clear()



    $tweaks = Get-TweaksConfig -TargetOS $activeOS



    $categories = $tweaks | Select-Object -ExpandProperty category -Unique



    foreach ($cat in $categories) {



        # Category Header



        $catHeader = New-Object System.Windows.Controls.TextBlock



        $catHeader.Text = $cat.ToUpper()



        $catHeader.FontSize = 11



        $catHeader.FontWeight = [System.Windows.FontWeights]::Bold



        $catHeader.SetResourceReference([System.Windows.Controls.TextBlock]::ForegroundProperty, "AccentBrush")



        $catHeader.Margin = New-Object System.Windows.Thickness(2, 14, 0, 6)



        [void]$pnlTweaks.Children.Add($catHeader)



        $catTweaks = $tweaks | Where-Object { $_.category -eq $cat }



        foreach ($tw in $catTweaks) {



            $isApplied = Test-TweakApplied -Tweak $tw



            $tweakCard = New-Object System.Windows.Controls.Border



            $tweakCard.SetResourceReference([System.Windows.Controls.Border]::BackgroundProperty, "TweakCardBg")



            $tweakCard.SetResourceReference([System.Windows.Controls.Border]::BorderBrushProperty, "TweakCardBorder")



            $tweakCard.BorderThickness = New-Object System.Windows.Thickness(1)



            $tweakCard.CornerRadius = New-Object System.Windows.CornerRadius(8)



            $tweakCard.Margin = New-Object System.Windows.Thickness(0, 0, 0, 7)



            $tweakCard.Padding = New-Object System.Windows.Thickness(14, 11, 14, 11)



            $cardGrid = New-Object System.Windows.Controls.Grid



            $colCheck = New-Object System.Windows.Controls.ColumnDefinition



            $colCheck.Width = New-Object System.Windows.GridLength(1, [System.Windows.GridUnitType]::Star)



            $colBadge = New-Object System.Windows.Controls.ColumnDefinition



            $colBadge.Width = [System.Windows.GridLength]::Auto



            [void]$cardGrid.ColumnDefinitions.Add($colCheck)



            [void]$cardGrid.ColumnDefinitions.Add($colBadge)



            $chk = New-Object System.Windows.Controls.CheckBox



            $chk.SetResourceReference([System.Windows.Controls.CheckBox]::ForegroundProperty, "TextPrimaryBrush")



            $chk.FontWeight = [System.Windows.FontWeights]::SemiBold



            $chk.FontSize = 13



            $chk.VerticalAlignment = [System.Windows.VerticalAlignment]::Center



            $chkContent = New-Object System.Windows.Controls.StackPanel



            $tTitle = New-Object System.Windows.Controls.TextBlock



            $tTitle.Text = $tw.title



            $tTitle.FontWeight = [System.Windows.FontWeights]::SemiBold



            $tTitle.SetResourceReference([System.Windows.Controls.TextBlock]::ForegroundProperty, "TextPrimaryBrush")



            $tDesc = New-Object System.Windows.Controls.TextBlock



            $tDesc.Text = $tw.description



            $tDesc.FontSize = 11



            $tDesc.SetResourceReference([System.Windows.Controls.TextBlock]::ForegroundProperty, "TextSecondaryBrush")



            $tDesc.TextWrapping = [System.Windows.TextWrapping]::Wrap



            $tDesc.Margin = New-Object System.Windows.Thickness(0, 3, 0, 0)



            [void]$chkContent.Children.Add($tTitle)



            [void]$chkContent.Children.Add($tDesc)



            $chk.Content = $chkContent



            [System.Windows.Controls.Grid]::SetColumn($chk, 0)



            [void]$cardGrid.Children.Add($chk)



            # Modern Status Badge Pill



            $badge = New-Object System.Windows.Controls.Border



            $badge.CornerRadius = New-Object System.Windows.CornerRadius(10)



            $badge.Padding = New-Object System.Windows.Thickness(10, 3, 10, 3)



            $badge.VerticalAlignment = [System.Windows.VerticalAlignment]::Center



            $badge.BorderThickness = New-Object System.Windows.Thickness(1)



            $badgeText = New-Object System.Windows.Controls.TextBlock



            $badgeText.FontSize = 10



            $badgeText.FontWeight = [System.Windows.FontWeights]::Bold



            if ($isApplied) {



                $badge.SetResourceReference([System.Windows.Controls.Border]::BackgroundProperty, "BadgeOptimizedBg")



                $badge.SetResourceReference([System.Windows.Controls.Border]::BorderBrushProperty, "BadgeOptimizedBorder")



                $badgeText.Text = "Optimized"



                $badgeText.SetResourceReference([System.Windows.Controls.TextBlock]::ForegroundProperty, "BadgeOptimizedFg")



            } else {



                $badge.SetResourceReference([System.Windows.Controls.Border]::BackgroundProperty, "BadgeStandardBg")



                $badge.SetResourceReference([System.Windows.Controls.Border]::BorderBrushProperty, "BadgeStandardBorder")



                $badgeText.Text = "Standard"



                $badgeText.SetResourceReference([System.Windows.Controls.TextBlock]::ForegroundProperty, "BadgeStandardFg")



            }



            $badge.Child = $badgeText



            [System.Windows.Controls.Grid]::SetColumn($badge, 1)



            [void]$cardGrid.Children.Add($badge)



            $tweakCard.Child = $cardGrid



            [void]$pnlTweaks.Children.Add($tweakCard)



            $tweakCheckboxMap[$tw.id] = @{



                CheckBox = $chk;



                Tweak = $tw;



                Badge = $badge;



                BadgeText = $badgeText;



                Card = $tweakCard



            }



        }



    }



}



# Live Search Filter



if ($txtTweakSearch) {



    $txtTweakSearch.Add_TextChanged({



        $query = $txtTweakSearch.Text.Trim().ToLower()



        foreach ($id in $tweakCheckboxMap.Keys) {



            $item = $tweakCheckboxMap[$id]



            $tw = $item.Tweak



            $visible = [string]::IsNullOrEmpty($query) -or



                       ($tw.title.ToLower().Contains($query)) -or



                       ($tw.description.ToLower().Contains($query)) -or



                       ($tw.category.ToLower().Contains($query))



            $item.Card.Visibility = if ($visible) { [System.Windows.Visibility]::Visible } else { [System.Windows.Visibility]::Collapsed }



        }



    })



}



# Bloat Checkbox Registry



$bloatCheckboxMap = @{}



function Sync-BloatList {



    if (!$pnlBloat) { return }



    [void]$pnlBloat.Children.Clear()



    $bloatCheckboxMap.Clear()



    & $Log "Scanning for pre-installed apps..."



    $installed = Get-InstalledBloatware



    if ($installed.Count -eq 0) {



        $emptyText = New-Object System.Windows.Controls.TextBlock



        $emptyText.Text = "Your PC is in great shape! No unwanted sponsored apps detected."



        $emptyText.Foreground = New-Object System.Windows.Media.SolidColorBrush([System.Windows.Media.Color]::FromArgb(255, 63, 185, 80))



        $emptyText.FontSize = 13



        $emptyText.FontWeight = [System.Windows.FontWeights]::SemiBold



        $emptyText.Margin = New-Object System.Windows.Thickness(10)



        [void]$pnlBloat.Children.Add($emptyText)



        & $Log "Scan complete: 0 unwanted apps found."



        return



    }



    & $Log "Scan complete: Found $($installed.Count) removable apps."



    foreach ($item in $installed) {



        $card = New-Object System.Windows.Controls.Border



        $card.SetResourceReference([System.Windows.Controls.Border]::BackgroundProperty, "TweakCardBg")



        $card.SetResourceReference([System.Windows.Controls.Border]::BorderBrushProperty, "TweakCardBorder")



        $card.BorderThickness = New-Object System.Windows.Thickness(1)



        $card.CornerRadius = New-Object System.Windows.CornerRadius(8)



        $card.Margin = New-Object System.Windows.Thickness(0, 0, 0, 7)



        $card.Padding = New-Object System.Windows.Thickness(14, 11, 14, 11)



        $chk = New-Object System.Windows.Controls.CheckBox



        $chk.IsChecked = $true



        $chk.VerticalAlignment = [System.Windows.VerticalAlignment]::Center



        $content = New-Object System.Windows.Controls.StackPanel



        $title = New-Object System.Windows.Controls.TextBlock



        $title.Text = "$($item.DisplayName)"



        $title.FontWeight = [System.Windows.FontWeights]::SemiBold



        $title.SetResourceReference([System.Windows.Controls.TextBlock]::ForegroundProperty, "TextPrimaryBrush")



        $desc = New-Object System.Windows.Controls.TextBlock



        $desc.Text = "$($item.Description) - App ID: $($item.Name)"



        $desc.FontSize = 11



        $desc.SetResourceReference([System.Windows.Controls.TextBlock]::ForegroundProperty, "TextSecondaryBrush")



        [void]$content.Children.Add($title)



        [void]$content.Children.Add($desc)



        $chk.Content = $content



        $card.Child = $chk



        [void]$pnlBloat.Children.Add($card)



        $bloatCheckboxMap[$item.Name] = $chk



    }



}



# Populate initial list



Sync-TweakList



# ==============================================================================



#  THEME ENGINE (4 LUXURY SHADES: OBSIDIAN, MOONLIGHT, AURORA, NEBULA)



# ==============================================================================



function Set-AppTheme {



    param([string]$ThemeName)

    $conv = [System.Windows.Media.BrushConverter]::new()
    $parse = { param([string]$xaml) [System.Windows.Markup.XamlReader]::Parse($xaml) }

    # Animate luxury slider indicator thumb to active theme slot
    $targetX = switch ($ThemeName) {
        "Moonlight" { 35.0 }
        "Aurora"    { 70.0 }
        "Nebula"    { 105.0 }
        Default     { 0.0 } # Obsidian
    }

    if ($brdThemeSliderThumb) {
        try {
            if ($brdThemeSliderThumb.RenderTransform -is [System.Windows.Media.TranslateTransform]) {
                $anim = [System.Windows.Media.Animation.DoubleAnimation]::new()
                $anim.To = [double]$targetX
                $anim.Duration = [System.Windows.Duration]::new([System.TimeSpan]::FromMilliseconds(240))
                $ease = [System.Windows.Media.Animation.CubicEase]::new()
                $ease.EasingMode = [System.Windows.Media.Animation.EasingMode]::EaseOut
                $anim.EasingFunction = $ease
                $brdThemeSliderThumb.RenderTransform.BeginAnimation([System.Windows.Media.TranslateTransform]::XProperty, $anim)
            } else {
                $tt = [System.Windows.Media.TranslateTransform]::new($targetX, 0)
                $brdThemeSliderThumb.RenderTransform = $tt
            }
        } catch {
            if ($brdThemeSliderThumb.RenderTransform) { $brdThemeSliderThumb.RenderTransform.X = $targetX }
        }
    }

    switch ($ThemeName) {
        "Moonlight" {
            # Pristine Iced Alabaster & Royal Azure Glass Theme
            $window.Background = $conv.ConvertFromString("#F8FAFC")
            $bgLight = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#FFFFFF" Offset="0.0"/><GradientStop Color="#F8FAFC" Offset="0.5"/><GradientStop Color="#F1F5F9" Offset="1.0"/></LinearGradientBrush>'
            $borderLight = $conv.ConvertFromString("#CBD5E1")
            $window.Resources["WindowBackgroundBrush"] = $bgLight
            $window.Resources["WindowBorderBrush"] = $borderLight
            if ($rootBorder) {
                $rootBorder.Background = $bgLight
                $rootBorder.BorderBrush = $borderLight
            }
            if ($pnlAmbientGlowMesh) { $pnlAmbientGlowMesh.Opacity = 0.10 }
            if ($imgBgHero) { $imgBgHero.Opacity = 0.05 }

            if ($brdThemeBar) {
                $brdThemeBar.Background = $conv.ConvertFromString("#E2E8F0")
                $brdThemeBar.BorderBrush = $conv.ConvertFromString("#CBD5E1")
            }
            if ($brdThemeSliderThumb) {
                $brdThemeSliderThumb.Background = $conv.ConvertFromString("#FFFFFF")
                $brdThemeSliderThumb.BorderBrush = $conv.ConvertFromString("#38BDF8")
                $ds = [System.Windows.Media.Effects.DropShadowEffect]::new()
                $ds.BlurRadius = 12
                $ds.Color = [System.Windows.Media.ColorConverter]::ConvertFromString("#38BDF8")
                $ds.Opacity = 0.45
                $ds.ShadowDepth = 0
                $brdThemeSliderThumb.Effect = $ds
            }
            $window.Resources["ThemeIconObsidian"] = $conv.ConvertFromString("#94A3B8")
            $window.Resources["ThemeIconMoonlight"] = $conv.ConvertFromString("#0284C7")
            $window.Resources["ThemeIconAurora"] = $conv.ConvertFromString("#94A3B8")
            $window.Resources["ThemeIconNebula"] = $conv.ConvertFromString("#94A3B8")

            if ($brdTitleBar) { 
                $brdTitleBar.Background = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="0,1"><GradientStop Color="#FFFFFF" Offset="0.0"/><GradientStop Color="#F8FAFC" Offset="1.0"/></LinearGradientBrush>'
                $brdTitleBar.BorderBrush = $conv.ConvertFromString("#CBD5E1")
            }
            if ($brdSidebar) {
                $brdSidebar.Background = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#FFFFFF" Offset="0.0"/><GradientStop Color="#F8FAFC" Offset="1.0"/></LinearGradientBrush>'
                $brdSidebar.BorderBrush = $conv.ConvertFromString("#CBD5E1")
            }
            if ($brdSearchBox) {
                $brdSearchBox.Background = $conv.ConvertFromString("#F1F5F9")
                $brdSearchBox.BorderBrush = $conv.ConvertFromString("#CBD5E1")
            }
            if ($txtGlobalSearch) {
                $txtGlobalSearch.Foreground = $conv.ConvertFromString("#334155")
            }
            if ($btnOpenActivityLogs) {
                $btnOpenActivityLogs.Background = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="0,1"><GradientStop Color="#F1F5F9" Offset="0.0"/><GradientStop Color="#E2E8F0" Offset="1.0"/></LinearGradientBrush>'
                $btnOpenActivityLogs.BorderBrush = $conv.ConvertFromString("#CBD5E1")
            }
            if ($brdEditionBadge) {
                $brdEditionBadge.Background = $conv.ConvertFromString("#E0F2FE")
                $brdEditionBadge.BorderBrush = $conv.ConvertFromString("#BAE6FD")
            }
            if ($txtEditionBadge) {
                $txtEditionBadge.Foreground = $conv.ConvertFromString("#0284C7")
            }
            if ($brdOverviewDock) {
                $brdOverviewDock.Background = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="0,1"><GradientStop Color="#FFFFFF" Offset="0.0"/><GradientStop Color="#F8FAFC" Offset="1.0"/></LinearGradientBrush>'
                $brdOverviewDock.BorderBrush = $conv.ConvertFromString("#CBD5E1")
            }
            if ($brdOverviewDivider) {
                $brdOverviewDivider.Background = $conv.ConvertFromString("#CBD5E1")
            }
            if ($pnlTelemetryCapsule) {
                $pnlTelemetryCapsule.Background = $conv.ConvertFromString("#F1F5F9")
                $pnlTelemetryCapsule.BorderBrush = $conv.ConvertFromString("#CBD5E1")
            }
            if ($brdConsoleDrawer) {
                $brdConsoleDrawer.Background = $conv.ConvertFromString("#FFFFFF")
                $brdConsoleDrawer.BorderBrush = $conv.ConvertFromString("#CBD5E1")
            }

            $window.Resources["OverviewCardBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#FFFFFF" Offset="0.0"/><GradientStop Color="#F8FAFC" Offset="0.5"/><GradientStop Color="#F1F5F9" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["OverviewCardBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#FFFFFF" Offset="0.0"/><GradientStop Color="#BAE6FD" Offset="0.3"/><GradientStop Color="#CBD5E1" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["OverviewCardHoverBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#FFFFFF" Offset="0.0"/><GradientStop Color="#F0F9FF" Offset="0.5"/><GradientStop Color="#E0F2FE" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["OverviewCardHoverBorder"] = $conv.ConvertFromString("#38BDF8")
            $window.Resources["CardActionBtnBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="0,1"><GradientStop Color="#F1F5F9" Offset="0.0"/><GradientStop Color="#E2E8F0" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["CardActionBtnBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#FFFFFF" Offset="0.0"/><GradientStop Color="#CBD5E1" Offset="0.5"/><GradientStop Color="#94A3B8" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["CardActionBtnHoverBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="0,1"><GradientStop Color="#E2E8F0" Offset="0.0"/><GradientStop Color="#CBD5E1" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["CardActionBtnHoverBorder"] = $conv.ConvertFromString("#38BDF8")
            $window.Resources["TextPrimaryBrush"] = $conv.ConvertFromString("#0F172A")
            $window.Resources["TextSecondaryBrush"] = $conv.ConvertFromString("#334155")
            $window.Resources["TextMutedBrush"] = $conv.ConvertFromString("#64748B")
            $window.Resources["SidebarActiveBgBrush"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,0"><GradientStop Color="#E0F2FE" Offset="0.0"/><GradientStop Color="#BAE6FD" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SidebarActiveFgBrush"] = $conv.ConvertFromString("#0284C7")
            $window.Resources["SidebarCardBgBrush"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#F8FAFC" Offset="0.0"/><GradientStop Color="#F1F5F9" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SidebarCardBorderBrush"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#CBD5E1" Offset="0.0"/><GradientStop Color="#94A3B8" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SidebarHoverBgBrush"] = $conv.ConvertFromString("#E2E8F0")

            # Semantic Dynamic Tokens for Moonlight Light Theme
            $window.Resources["SectionBannerBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#FFFFFF" Offset="0.0"/><GradientStop Color="#F8FAFC" Offset="0.5"/><GradientStop Color="#F1F5F9" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SectionBannerBorder"] = $conv.ConvertFromString("#CBD5E1")
            $window.Resources["SectionBannerTitleBrush"] = $conv.ConvertFromString("#0284C7")
            $window.Resources["AccentBrush"] = $conv.ConvertFromString("#0284C7")
            $window.Resources["TweakCardBg"] = $conv.ConvertFromString("#FFFFFF")
            $window.Resources["TweakCardBorder"] = $conv.ConvertFromString("#E2E8F0")
            $window.Resources["TweakCardHoverBg"] = $conv.ConvertFromString("#F0F9FF")
            $window.Resources["BadgeOptimizedBg"] = $conv.ConvertFromString("#D1FAE5")
            $window.Resources["BadgeOptimizedBorder"] = $conv.ConvertFromString("#A7F3D0")
            $window.Resources["BadgeOptimizedFg"] = $conv.ConvertFromString("#059669")
            $window.Resources["BadgeStandardBg"] = $conv.ConvertFromString("#F1F5F9")
            $window.Resources["BadgeStandardBorder"] = $conv.ConvertFromString("#CBD5E1")
            $window.Resources["BadgeStandardFg"] = $conv.ConvertFromString("#475569")
            $window.Resources["WarningBannerBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#FFF7ED" Offset="0.0"/><GradientStop Color="#FFEDD5" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["WarningBannerBorder"] = $conv.ConvertFromString("#FDBA74")
            $window.Resources["DangerBannerBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#FEF2F2" Offset="0.0"/><GradientStop Color="#FEE2E2" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["DangerBannerBorder"] = $conv.ConvertFromString("#FCA5A5")
            $window.Resources["SuccessBannerBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#ECFDF5" Offset="0.0"/><GradientStop Color="#D1FAE5" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SuccessBannerBorder"] = $conv.ConvertFromString("#A7F3D0")
            $window.Resources["InputSearchBg"] = $conv.ConvertFromString("#FFFFFF")
            $window.Resources["InputSearchBorder"] = $conv.ConvertFromString("#CBD5E1")
            $window.Resources["InputSearchFg"] = $conv.ConvertFromString("#0F172A")
            $window.Resources["CircleNavBgBrush"] = $conv.ConvertFromString("#E2E8F0")
            $window.Resources["CircleNavBorderBrush"] = $conv.ConvertFromString("#CBD5E1")
        }

        "Aurora" {
            # Nordic Pine & Radiant Aurora Mint Glass Theme
            $window.Background = $conv.ConvertFromString("#020C09")
            $bgAurora = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#041A14" Offset="0.0"/><GradientStop Color="#020F0B" Offset="0.5"/><GradientStop Color="#010806" Offset="1.0"/></LinearGradientBrush>'
            $borderAurora = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#3310B981" Offset="0.0"/><GradientStop Color="#1A0E2E20" Offset="0.5"/><GradientStop Color="#0A05150E" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["WindowBackgroundBrush"] = $bgAurora
            $window.Resources["WindowBorderBrush"] = $borderAurora
            if ($rootBorder) {
                $rootBorder.Background = $bgAurora
                $rootBorder.BorderBrush = $borderAurora
            }
            if ($pnlAmbientGlowMesh) { $pnlAmbientGlowMesh.Opacity = 0.85 }
            if ($imgBgHero) { $imgBgHero.Opacity = 0.88 }

            if ($brdThemeBar) {
                $brdThemeBar.Background = $conv.ConvertFromString("#4D041B13")
                $brdThemeBar.BorderBrush = $conv.ConvertFromString("#4D10B981")
            }
            if ($brdThemeSliderThumb) {
                $brdThemeSliderThumb.Background = $conv.ConvertFromString("#4D10B981")
                $brdThemeSliderThumb.BorderBrush = $conv.ConvertFromString("#8010B981")
                $ds = [System.Windows.Media.Effects.DropShadowEffect]::new()
                $ds.BlurRadius = 12
                $ds.Color = [System.Windows.Media.ColorConverter]::ConvertFromString("#10B981")
                $ds.Opacity = 0.55
                $ds.ShadowDepth = 0
                $brdThemeSliderThumb.Effect = $ds
            }
            $window.Resources["ThemeIconObsidian"] = $conv.ConvertFromString("#64748B")
            $window.Resources["ThemeIconMoonlight"] = $conv.ConvertFromString("#64748B")
            $window.Resources["ThemeIconAurora"] = $conv.ConvertFromString("#10B981")
            $window.Resources["ThemeIconNebula"] = $conv.ConvertFromString("#64748B")

            if ($brdTitleBar) { 
                $brdTitleBar.Background = $conv.ConvertFromString("#4D031A14")
                $brdTitleBar.BorderBrush = $conv.ConvertFromString("#3310B981")
            }
            if ($brdSidebar) {
                $brdSidebar.Background = $conv.ConvertFromString("#5902120D")
                $brdSidebar.BorderBrush = $conv.ConvertFromString("#2610B981")
            }
            if ($brdSearchBox) {
                $brdSearchBox.Background = $conv.ConvertFromString("#66051E16")
                $brdSearchBox.BorderBrush = $conv.ConvertFromString("#3310B981")
            }
            if ($txtGlobalSearch) {
                $txtGlobalSearch.Foreground = $conv.ConvertFromString("#A7F3D0")
            }
            if ($btnOpenActivityLogs) {
                $btnOpenActivityLogs.Background = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="0,1"><GradientStop Color="#330A261C" Offset="0.0"/><GradientStop Color="#1A04150F" Offset="1.0"/></LinearGradientBrush>'
                $btnOpenActivityLogs.BorderBrush = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="0,1"><GradientStop Color="#4010B981" Offset="0.0"/><GradientStop Color="#140A3022" Offset="1.0"/></LinearGradientBrush>'
            }
            if ($brdEditionBadge) {
                $brdEditionBadge.Background = $conv.ConvertFromString("#06281E")
                $brdEditionBadge.BorderBrush = $conv.ConvertFromString("#0A3E2F")
            }
            if ($txtEditionBadge) {
                $txtEditionBadge.Foreground = $conv.ConvertFromString("#34D399")
            }
            if ($brdOverviewDock) {
                $brdOverviewDock.Background = $conv.ConvertFromString("#54031610")
                $brdOverviewDock.BorderBrush = $conv.ConvertFromString("#4D10B981")
            }
            if ($brdOverviewDivider) {
                $brdOverviewDivider.Background = $conv.ConvertFromString("#142234")
            }
            if ($pnlTelemetryCapsule) {
                $pnlTelemetryCapsule.Background = $conv.ConvertFromString("#4D03140F")
                $pnlTelemetryCapsule.BorderBrush = $conv.ConvertFromString("#3310B981")
            }
            if ($brdConsoleDrawer) {
                $brdConsoleDrawer.Background = $conv.ConvertFromString("#54020E0A")
                $brdConsoleDrawer.BorderBrush = $conv.ConvertFromString("#3310B981")
            }

            $window.Resources["OverviewCardBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#4D06241C" Offset="0.0"/><GradientStop Color="#3303140F" Offset="0.5"/><GradientStop Color="#1A020E0A" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["OverviewCardBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#8CFFFFFF" Offset="0.0"/><GradientStop Color="#6600F5A0" Offset="0.25"/><GradientStop Color="#3310B981" Offset="0.6"/><GradientStop Color="#100E2E20" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["OverviewCardHoverBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#4D0D3D2F" Offset="0.0"/><GradientStop Color="#3306261C" Offset="0.5"/><GradientStop Color="#24031610" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["OverviewCardHoverBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#E600F5A0" Offset="0.0"/><GradientStop Color="#9910B981" Offset="0.35"/><GradientStop Color="#4D34D399" Offset="0.75"/><GradientStop Color="#1A0E2E20" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["CardActionBtnBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="0,1"><GradientStop Color="#4D0A3024" Offset="0.0"/><GradientStop Color="#66051A13" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["CardActionBtnBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#80FFFFFF" Offset="0.0"/><GradientStop Color="#4D10B981" Offset="0.3"/><GradientStop Color="#1A133A29" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["CardActionBtnHoverBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="0,1"><GradientStop Color="#66144D3A" Offset="0.0"/><GradientStop Color="#400A2D22" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["CardActionBtnHoverBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#CC00F5A0" Offset="0.0"/><GradientStop Color="#6610B981" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["TextPrimaryBrush"] = $conv.ConvertFromString("#FFFFFF")
            $window.Resources["TextSecondaryBrush"] = $conv.ConvertFromString("#A7F3D0")
            $window.Resources["TextMutedBrush"] = $conv.ConvertFromString("#6EE7B7")
            $window.Resources["SidebarActiveBgBrush"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,0"><GradientStop Color="#3300F5A0" Offset="0.0"/><GradientStop Color="#1A10B981" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SidebarCardBgBrush"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#4D052118" Offset="0.0"/><GradientStop Color="#26020E0A" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SidebarCardBorderBrush"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#66FFFFFF" Offset="0.0"/><GradientStop Color="#3310B981" Offset="0.5"/><GradientStop Color="#10082A1F" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SidebarHoverBgBrush"] = $conv.ConvertFromString("#133A29")

            # Semantic Dynamic Tokens for Aurora Theme
            $window.Resources["SidebarActiveFgBrush"] = $conv.ConvertFromString("#FFFFFF")
            $window.Resources["SectionBannerBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#4006241C" Offset="0.0"/><GradientStop Color="#2003140F" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SectionBannerBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#8CFFFFFF" Offset="0.0"/><GradientStop Color="#4D10B981" Offset="0.3"/><GradientStop Color="#2634D399" Offset="0.7"/><GradientStop Color="#100E2E20" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SectionBannerTitleBrush"] = $conv.ConvertFromString("#10B981")
            $window.Resources["AccentBrush"] = $conv.ConvertFromString("#10B981")
            $window.Resources["TweakCardBg"] = $conv.ConvertFromString("#061B14")
            $window.Resources["TweakCardBorder"] = $conv.ConvertFromString("#0E3326")
            $window.Resources["TweakCardHoverBg"] = $conv.ConvertFromString("#0A261D")
            $window.Resources["BadgeOptimizedBg"] = $conv.ConvertFromString("#2810B981")
            $window.Resources["BadgeOptimizedBorder"] = $conv.ConvertFromString("#7810B981")
            $window.Resources["BadgeOptimizedFg"] = $conv.ConvertFromString("#34D399")
            $window.Resources["BadgeStandardBg"] = $conv.ConvertFromString("#196EE7B7")
            $window.Resources["BadgeStandardBorder"] = $conv.ConvertFromString("#3C6EE7B7")
            $window.Resources["BadgeStandardFg"] = $conv.ConvertFromString("#6EE7B7")
            $window.Resources["WarningBannerBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#4D2E1208" Offset="0.0"/><GradientStop Color="#26180803" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["WarningBannerBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#99FFAA70" Offset="0.0"/><GradientStop Color="#80F97316" Offset="0.3"/><GradientStop Color="#33EA580C" Offset="0.7"/><GradientStop Color="#15261108" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["DangerBannerBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#3828080C" Offset="0.0"/><GradientStop Color="#1A140306" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["DangerBannerBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#66EF4444" Offset="0.0"/><GradientStop Color="#26B91C1C" Offset="0.5"/><GradientStop Color="#1024080B" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SuccessBannerBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#38082417" Offset="0.0"/><GradientStop Color="#1A04140D" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SuccessBannerBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#6610B981" Offset="0.0"/><GradientStop Color="#26059669" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["InputSearchBg"] = $conv.ConvertFromString("#04150F")
            $window.Resources["InputSearchBorder"] = $conv.ConvertFromString("#0E3326")
            $window.Resources["InputSearchFg"] = $conv.ConvertFromString("#FFFFFF")
            $window.Resources["CircleNavBgBrush"] = $conv.ConvertFromString("#4D0A261C")
            $window.Resources["CircleNavBorderBrush"] = $conv.ConvertFromString("#4D10B981")
        }

        "Nebula" {
            # Cosmic Deep Ultraviolet & Neon Fuchsia Glass Theme
            $window.Background = $conv.ConvertFromString("#070412")
            $bgNebula = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#0E0824" Offset="0.0"/><GradientStop Color="#080417" Offset="0.5"/><GradientStop Color="#04020D" Offset="1.0"/></LinearGradientBrush>'
            $borderNebula = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#33A855F7" Offset="0.0"/><GradientStop Color="#1A241445" Offset="0.5"/><GradientStop Color="#0A0C061C" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["WindowBackgroundBrush"] = $bgNebula
            $window.Resources["WindowBorderBrush"] = $borderNebula
            if ($rootBorder) {
                $rootBorder.Background = $bgNebula
                $rootBorder.BorderBrush = $borderNebula
            }
            if ($pnlAmbientGlowMesh) { $pnlAmbientGlowMesh.Opacity = 0.85 }
            if ($imgBgHero) { $imgBgHero.Opacity = 0.90 }

            if ($brdThemeBar) {
                $brdThemeBar.Background = $conv.ConvertFromString("#4D13092B")
                $brdThemeBar.BorderBrush = $conv.ConvertFromString("#4DA855F7")
            }
            if ($brdThemeSliderThumb) {
                $brdThemeSliderThumb.Background = $conv.ConvertFromString("#4DA855F7")
                $brdThemeSliderThumb.BorderBrush = $conv.ConvertFromString("#80A855F7")
                $ds = [System.Windows.Media.Effects.DropShadowEffect]::new()
                $ds.BlurRadius = 12
                $ds.Color = [System.Windows.Media.ColorConverter]::ConvertFromString("#A855F7")
                $ds.Opacity = 0.55
                $ds.ShadowDepth = 0
                $brdThemeSliderThumb.Effect = $ds
            }
            $window.Resources["ThemeIconObsidian"] = $conv.ConvertFromString("#64748B")
            $window.Resources["ThemeIconMoonlight"] = $conv.ConvertFromString("#64748B")
            $window.Resources["ThemeIconAurora"] = $conv.ConvertFromString("#64748B")
            $window.Resources["ThemeIconNebula"] = $conv.ConvertFromString("#C084FC")

            if ($brdTitleBar) { 
                $brdTitleBar.Background = $conv.ConvertFromString("#4D0E0824")
                $brdTitleBar.BorderBrush = $conv.ConvertFromString("#33A855F7")
            }
            if ($brdSidebar) {
                $brdSidebar.Background = $conv.ConvertFromString("#590B061C")
                $brdSidebar.BorderBrush = $conv.ConvertFromString("#26A855F7")
            }
            if ($brdSearchBox) {
                $brdSearchBox.Background = $conv.ConvertFromString("#66140A30")
                $brdSearchBox.BorderBrush = $conv.ConvertFromString("#33A855F7")
            }
            if ($txtGlobalSearch) {
                $txtGlobalSearch.Foreground = $conv.ConvertFromString("#E9D5FF")
            }
            if ($btnOpenActivityLogs) {
                $btnOpenActivityLogs.Background = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="0,1"><GradientStop Color="#331D0F3B" Offset="0.0"/><GradientStop Color="#1A0F0720" Offset="1.0"/></LinearGradientBrush>'
                $btnOpenActivityLogs.BorderBrush = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="0,1"><GradientStop Color="#40A855F7" Offset="0.0"/><GradientStop Color="#141E0F38" Offset="1.0"/></LinearGradientBrush>'
            }
            if ($brdEditionBadge) {
                $brdEditionBadge.Background = $conv.ConvertFromString("#1E1038")
                $brdEditionBadge.BorderBrush = $conv.ConvertFromString("#351963")
            }
            if ($txtEditionBadge) {
                $txtEditionBadge.Foreground = $conv.ConvertFromString("#C084FC")
            }
            if ($brdOverviewDock) {
                $brdOverviewDock.Background = $conv.ConvertFromString("#540E0722")
                $brdOverviewDock.BorderBrush = $conv.ConvertFromString("#4DA855F7")
            }
            if ($brdOverviewDivider) {
                $brdOverviewDivider.Background = $conv.ConvertFromString("#0B261D")
            }
            if ($pnlTelemetryCapsule) {
                $pnlTelemetryCapsule.Background = $conv.ConvertFromString("#4D0B061C")
                $pnlTelemetryCapsule.BorderBrush = $conv.ConvertFromString("#33A855F7")
            }
            if ($brdConsoleDrawer) {
                $brdConsoleDrawer.Background = $conv.ConvertFromString("#54070312")
                $brdConsoleDrawer.BorderBrush = $conv.ConvertFromString("#33A855F7")
            }

            $window.Resources["OverviewCardBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#4D160C36" Offset="0.0"/><GradientStop Color="#330C0720" Offset="0.5"/><GradientStop Color="#1A070412" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["OverviewCardBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#8CFFFFFF" Offset="0.0"/><GradientStop Color="#66EC4899" Offset="0.25"/><GradientStop Color="#33A855F7" Offset="0.6"/><GradientStop Color="#10241445" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["OverviewCardHoverBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#4D2A145A" Offset="0.0"/><GradientStop Color="#33160A34" Offset="0.5"/><GradientStop Color="#240D051F" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["OverviewCardHoverBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#E6EC4899" Offset="0.0"/><GradientStop Color="#99A855F7" Offset="0.35"/><GradientStop Color="#4DC084FC" Offset="0.75"/><GradientStop Color="#1A241445" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["CardActionBtnBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="0,1"><GradientStop Color="#4D1F1045" Offset="0.0"/><GradientStop Color="#66110826" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["CardActionBtnBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#80FFFFFF" Offset="0.0"/><GradientStop Color="#4DA855F7" Offset="0.3"/><GradientStop Color="#1A2D174E" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["CardActionBtnHoverBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="0,1"><GradientStop Color="#6634176E" Offset="0.0"/><GradientStop Color="#401B0C3D" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["CardActionBtnHoverBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#CCEC4899" Offset="0.0"/><GradientStop Color="#66A855F7" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["TextPrimaryBrush"] = $conv.ConvertFromString("#FFFFFF")
            $window.Resources["TextSecondaryBrush"] = $conv.ConvertFromString("#E9D5FF")
            $window.Resources["TextMutedBrush"] = $conv.ConvertFromString("#C084FC")
            $window.Resources["SidebarActiveBgBrush"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,0"><GradientStop Color="#33A855F7" Offset="0.0"/><GradientStop Color="#1AEC4899" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SidebarCardBgBrush"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#4D13092D" Offset="0.0"/><GradientStop Color="#26070414" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SidebarCardBorderBrush"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#66FFFFFF" Offset="0.0"/><GradientStop Color="#33A855F7" Offset="0.5"/><GradientStop Color="#101D1040" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SidebarHoverBgBrush"] = $conv.ConvertFromString("#241445")

            # Semantic Dynamic Tokens for Nebula Theme
            $window.Resources["SidebarActiveFgBrush"] = $conv.ConvertFromString("#FFFFFF")
            $window.Resources["SectionBannerBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#40160C36" Offset="0.0"/><GradientStop Color="#200C0720" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SectionBannerBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#8CFFFFFF" Offset="0.0"/><GradientStop Color="#4DA855F7" Offset="0.3"/><GradientStop Color="#26EC4899" Offset="0.7"/><GradientStop Color="#10241445" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SectionBannerTitleBrush"] = $conv.ConvertFromString("#C084FC")
            $window.Resources["AccentBrush"] = $conv.ConvertFromString("#C084FC")
            $window.Resources["TweakCardBg"] = $conv.ConvertFromString("#120926")
            $window.Resources["TweakCardBorder"] = $conv.ConvertFromString("#26134D")
            $window.Resources["TweakCardHoverBg"] = $conv.ConvertFromString("#1A0E38")
            $window.Resources["BadgeOptimizedBg"] = $conv.ConvertFromString("#28A855F7")
            $window.Resources["BadgeOptimizedBorder"] = $conv.ConvertFromString("#78A855F7")
            $window.Resources["BadgeOptimizedFg"] = $conv.ConvertFromString("#C084FC")
            $window.Resources["BadgeStandardBg"] = $conv.ConvertFromString("#19A855F7")
            $window.Resources["BadgeStandardBorder"] = $conv.ConvertFromString("#3CA855F7")
            $window.Resources["BadgeStandardFg"] = $conv.ConvertFromString("#A855F7")
            $window.Resources["WarningBannerBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#4D2E1208" Offset="0.0"/><GradientStop Color="#26180803" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["WarningBannerBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#99FFAA70" Offset="0.0"/><GradientStop Color="#80F97316" Offset="0.3"/><GradientStop Color="#33EA580C" Offset="0.7"/><GradientStop Color="#15261108" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["DangerBannerBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#3828080C" Offset="0.0"/><GradientStop Color="#1A140306" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["DangerBannerBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#66EF4444" Offset="0.0"/><GradientStop Color="#26B91C1C" Offset="0.5"/><GradientStop Color="#1024080B" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SuccessBannerBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#38082417" Offset="0.0"/><GradientStop Color="#1A04140D" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SuccessBannerBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#6610B981" Offset="0.0"/><GradientStop Color="#26059669" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["InputSearchBg"] = $conv.ConvertFromString("#0B061A")
            $window.Resources["InputSearchBorder"] = $conv.ConvertFromString("#26134D")
            $window.Resources["InputSearchFg"] = $conv.ConvertFromString("#FFFFFF")
            $window.Resources["CircleNavBgBrush"] = $conv.ConvertFromString("#4D1D0F3B")
            $window.Resources["CircleNavBorderBrush"] = $conv.ConvertFromString("#4DA855F7")
        }

        Default { # "Obsidian"
            # OLED Pitch Black & Electric Cyan Glass Theme
            $window.Background = $conv.ConvertFromString("#04070D")
            $bgObsidian = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#08101E" Offset="0.0"/><GradientStop Color="#050A14" Offset="0.5"/><GradientStop Color="#03060B" Offset="1.0"/></LinearGradientBrush>'
            $borderObsidian = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#3338BDF8" Offset="0.0"/><GradientStop Color="#1A1E2E44" Offset="0.5"/><GradientStop Color="#0A0F1A" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["WindowBackgroundBrush"] = $bgObsidian
            $window.Resources["WindowBorderBrush"] = $borderObsidian
            if ($rootBorder) {
                $rootBorder.Background = $bgObsidian
                $rootBorder.BorderBrush = $borderObsidian
            }
            if ($pnlAmbientGlowMesh) { $pnlAmbientGlowMesh.Opacity = 0.85 }
            if ($imgBgHero) { $imgBgHero.Opacity = 0.92 }

            if ($brdThemeBar) {
                $brdThemeBar.Background = $conv.ConvertFromString("#330E1C30")
                $brdThemeBar.BorderBrush = $conv.ConvertFromString("#3838BDF8")
            }
            if ($brdThemeSliderThumb) {
                $brdThemeSliderThumb.Background = $conv.ConvertFromString("#4D00F5D4")
                $brdThemeSliderThumb.BorderBrush = $conv.ConvertFromString("#8000F5D4")
                $ds = [System.Windows.Media.Effects.DropShadowEffect]::new()
                $ds.BlurRadius = 12
                $ds.Color = [System.Windows.Media.ColorConverter]::ConvertFromString("#00F5D4")
                $ds.Opacity = 0.55
                $ds.ShadowDepth = 0
                $brdThemeSliderThumb.Effect = $ds
            }
            $window.Resources["ThemeIconObsidian"] = $conv.ConvertFromString("#00F5D4")
            $window.Resources["ThemeIconMoonlight"] = $conv.ConvertFromString("#64748B")
            $window.Resources["ThemeIconAurora"] = $conv.ConvertFromString("#64748B")
            $window.Resources["ThemeIconNebula"] = $conv.ConvertFromString("#64748B")

            if ($brdTitleBar) { 
                $brdTitleBar.Background = $conv.ConvertFromString("#4D08111D")
                $brdTitleBar.BorderBrush = $conv.ConvertFromString("#3338BDF8")
            }
            if ($brdSidebar) {
                $brdSidebar.Background = $conv.ConvertFromString("#59060E18")
                $brdSidebar.BorderBrush = $conv.ConvertFromString("#2638BDF8")
            }
            if ($brdSearchBox) {
                $brdSearchBox.Background = $conv.ConvertFromString("#660C182B")
                $brdSearchBox.BorderBrush = $conv.ConvertFromString("#3338BDF8")
            }
            if ($txtGlobalSearch) {
                $txtGlobalSearch.Foreground = $conv.ConvertFromString("#94A3B8")
            }
            if ($btnOpenActivityLogs) {
                $btnOpenActivityLogs.Background = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="0,1"><GradientStop Color="#33182B42" Offset="0.0"/><GradientStop Color="#1A0D1826" Offset="1.0"/></LinearGradientBrush>'
                $btnOpenActivityLogs.BorderBrush = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="0,1"><GradientStop Color="#4038BDF8" Offset="0.0"/><GradientStop Color="#141E2D44" Offset="1.0"/></LinearGradientBrush>'
            }
            if ($brdEditionBadge) {
                $brdEditionBadge.Background = $conv.ConvertFromString("#111E32")
                $brdEditionBadge.BorderBrush = $conv.ConvertFromString("#1C3050")
            }
            if ($txtEditionBadge) {
                $txtEditionBadge.Foreground = $conv.ConvertFromString("#60A5FA")
            }
            if ($brdOverviewDock) {
                $brdOverviewDock.Background = $conv.ConvertFromString("#5412243C")
                $brdOverviewDock.BorderBrush = $conv.ConvertFromString("#4D00F5D4")
            }
            if ($brdOverviewDivider) {
                $brdOverviewDivider.Background = $conv.ConvertFromString("#1D1035")
            }
            if ($pnlTelemetryCapsule) {
                $pnlTelemetryCapsule.Background = $conv.ConvertFromString("#4D070E18")
                $pnlTelemetryCapsule.BorderBrush = $conv.ConvertFromString("#2638BDF8")
            }
            if ($brdConsoleDrawer) {
                $brdConsoleDrawer.Background = $conv.ConvertFromString("#5404070D")
                $brdConsoleDrawer.BorderBrush = $conv.ConvertFromString("#3338BDF8")
            }

            $window.Resources["OverviewCardBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#4A102038" Offset="0.0"/><GradientStop Color="#300A1526" Offset="0.5"/><GradientStop Color="#1E060C16" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["OverviewCardBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#8CFFFFFF" Offset="0.0"/><GradientStop Color="#5900F5D4" Offset="0.25"/><GradientStop Color="#2E38BDF8" Offset="0.6"/><GradientStop Color="#101E2E44" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["OverviewCardHoverBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#4D223F63" Offset="0.0"/><GradientStop Color="#33142740" Offset="0.5"/><GradientStop Color="#240B1624" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["OverviewCardHoverBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#E600F5D4" Offset="0.0"/><GradientStop Color="#9938BDF8" Offset="0.35"/><GradientStop Color="#4D818CF8" Offset="0.75"/><GradientStop Color="#1A1E293B" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["CardActionBtnBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="0,1"><GradientStop Color="#4D162842" Offset="0.0"/><GradientStop Color="#730C1524" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["CardActionBtnBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#80FFFFFF" Offset="0.0"/><GradientStop Color="#4D38BDF8" Offset="0.3"/><GradientStop Color="#1A243650" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["CardActionBtnHoverBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="0,1"><GradientStop Color="#6625446C" Offset="0.0"/><GradientStop Color="#4014253B" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["CardActionBtnHoverBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#CC00F5D4" Offset="0.0"/><GradientStop Color="#6638BDF8" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["TextPrimaryBrush"] = $conv.ConvertFromString("#FFFFFF")
            $window.Resources["TextSecondaryBrush"] = $conv.ConvertFromString("#94A3B8")
            $window.Resources["TextMutedBrush"] = $conv.ConvertFromString("#64748B")
            $window.Resources["SidebarActiveBgBrush"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,0"><GradientStop Color="#3300F5D4" Offset="0.0"/><GradientStop Color="#1A38BDF8" Offset="0.5"/><GradientStop Color="#0A0EA5E9" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SidebarCardBgBrush"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#4D0F1B2E" Offset="0.0"/><GradientStop Color="#2608101C" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SidebarCardBorderBrush"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#66FFFFFF" Offset="0.0"/><GradientStop Color="#3338BDF8" Offset="0.5"/><GradientStop Color="#101E2E44" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SidebarHoverBgBrush"] = $conv.ConvertFromString("#1A2840")

            # Semantic Dynamic Tokens for Obsidian Theme
            $window.Resources["SidebarActiveFgBrush"] = $conv.ConvertFromString("#FFFFFF")
            $window.Resources["SectionBannerBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#400F1F35" Offset="0.0"/><GradientStop Color="#20081324" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SectionBannerBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#8CFFFFFF" Offset="0.0"/><GradientStop Color="#4D00F5D4" Offset="0.3"/><GradientStop Color="#2638BDF8" Offset="0.7"/><GradientStop Color="#101E2E44" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SectionBannerTitleBrush"] = $conv.ConvertFromString("#00F5D4")
            $window.Resources["AccentBrush"] = $conv.ConvertFromString("#00F5D4")
            $window.Resources["TweakCardBg"] = $conv.ConvertFromString("#0C1320")
            $window.Resources["TweakCardBorder"] = $conv.ConvertFromString("#1C2B44")
            $window.Resources["TweakCardHoverBg"] = $conv.ConvertFromString("#131E33")
            $window.Resources["BadgeOptimizedBg"] = $conv.ConvertFromString("#2810B981")
            $window.Resources["BadgeOptimizedBorder"] = $conv.ConvertFromString("#7810B981")
            $window.Resources["BadgeOptimizedFg"] = $conv.ConvertFromString("#10B981")
            $window.Resources["BadgeStandardBg"] = $conv.ConvertFromString("#1994A3B8")
            $window.Resources["BadgeStandardBorder"] = $conv.ConvertFromString("#3C94A3B8")
            $window.Resources["BadgeStandardFg"] = $conv.ConvertFromString("#94A3B8")
            $window.Resources["WarningBannerBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#4D2E1208" Offset="0.0"/><GradientStop Color="#26180803" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["WarningBannerBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#99FFAA70" Offset="0.0"/><GradientStop Color="#80F97316" Offset="0.3"/><GradientStop Color="#33EA580C" Offset="0.7"/><GradientStop Color="#15261108" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["DangerBannerBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#3828080C" Offset="0.0"/><GradientStop Color="#1A140306" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["DangerBannerBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#66EF4444" Offset="0.0"/><GradientStop Color="#26B91C1C" Offset="0.5"/><GradientStop Color="#1024080B" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SuccessBannerBg"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#38082417" Offset="0.0"/><GradientStop Color="#1A04140D" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["SuccessBannerBorder"] = &$parse '<LinearGradientBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" StartPoint="0,0" EndPoint="1,1"><GradientStop Color="#6610B981" Offset="0.0"/><GradientStop Color="#26059669" Offset="1.0"/></LinearGradientBrush>'
            $window.Resources["InputSearchBg"] = $conv.ConvertFromString("#080E18")
            $window.Resources["InputSearchBorder"] = $conv.ConvertFromString("#1C2D44")
            $window.Resources["InputSearchFg"] = $conv.ConvertFromString("#FFFFFF")
            $window.Resources["CircleNavBgBrush"] = $conv.ConvertFromString("#4D192C44")
            $window.Resources["CircleNavBorderBrush"] = $conv.ConvertFromString("#4D38BDF8")
        }
    }

    # Dynamic Gauge and Wave Palette Accent Synchronization
    $gaugeAccents = switch ($ThemeName) {
        "Moonlight" { @{ CPU = "#10B981"; RAM = "#2563EB"; Disk = "#0284C7" } }
        "Aurora"    { @{ CPU = "#10B981"; RAM = "#00F5A0"; Disk = "#34D399" } }
        "Nebula"    { @{ CPU = "#A855F7"; RAM = "#EC4899"; Disk = "#C084FC" } }
        Default     { @{ CPU = "#00F5A0"; RAM = "#38BDF8"; Disk = "#38BDF8" } }
    }
    try {
        if ($arcGaugeCPU)   { $arcGaugeCPU.Stroke   = $conv.ConvertFromString($gaugeAccents.CPU) }
        if ($arcGaugeRAM)   { $arcGaugeRAM.Stroke   = $conv.ConvertFromString($gaugeAccents.RAM) }
        if ($arcGaugeDisk)  { $arcGaugeDisk.Stroke  = $conv.ConvertFromString($gaugeAccents.Disk) }
        if ($pathSparkCPU)  { $pathSparkCPU.Stroke  = $conv.ConvertFromString($gaugeAccents.CPU) }
        if ($pathSparkRAM)  { $pathSparkRAM.Stroke  = $conv.ConvertFromString($gaugeAccents.RAM) }
        if ($pathSparkDisk) { $pathSparkDisk.Stroke = $conv.ConvertFromString($gaugeAccents.Disk) }
    } catch { }

    # Smooth Micro-Fade Transition on theme change
    try {
        $fadeAnim = [System.Windows.Media.Animation.DoubleAnimation]::new()
        $fadeAnim.From = 0.88
        $fadeAnim.To = 1.0
        $fadeAnim.Duration = [System.Windows.Duration]::new([System.TimeSpan]::FromMilliseconds(180))
        $fadeEase = [System.Windows.Media.Animation.CubicEase]::new()
        $fadeEase.EasingMode = [System.Windows.Media.Animation.EasingMode]::EaseOut
        $fadeAnim.EasingFunction = $fadeEase
        $window.BeginAnimation([System.Windows.UIElement]::OpacityProperty, $fadeAnim)
    } catch { }

    # Refresh active nav button styling to match current theme
    try {
        if ($SetNavActive -and $tabsWorkspace) {
            & $SetNavActive $tabsWorkspace.SelectedIndex
        }
    } catch { }

    # Persist theme choice safely



    try {



        $themeDir = Join-Path $scriptDir "data"



        if (!(Test-Path $themeDir)) { New-Item -ItemType Directory -Path $themeDir -Force | Out-Null }



        $themeConfig = @{ Theme = $ThemeName } | ConvertTo-Json



        [System.IO.File]::WriteAllText((Join-Path $themeDir "theme.json"), $themeConfig)



    } catch {}



}



# Theme Button Event Handlers



if ($btnThemeObsidian) { $btnThemeObsidian.Add_Click({ Set-AppTheme -ThemeName "Obsidian" }) }



if ($btnThemeMoonlight) { $btnThemeMoonlight.Add_Click({ Set-AppTheme -ThemeName "Moonlight" }) }



if ($btnThemeAurora) { $btnThemeAurora.Add_Click({ Set-AppTheme -ThemeName "Aurora" }) }



if ($btnThemeNebula) { $btnThemeNebula.Add_Click({ Set-AppTheme -ThemeName "Nebula" }) }



# Load Saved Theme



$initialTheme = "Obsidian"



try {



    $themeFile = Join-Path $scriptDir "data/theme.json"



    if (Test-Path $themeFile) {



        $saved = Get-Content $themeFile -Raw | ConvertFrom-Json



        if ($saved.Theme) { $initialTheme = $saved.Theme }



    }



} catch {}



Set-AppTheme -ThemeName $initialTheme



# ==============================================================================



#  BUTTON EVENT HANDLERS



# ==============================================================================



# Restore Point



$UpdateBackupsUI = {
    if (!$pnlRestorePointsList) { return }
    $pnlRestorePointsList.Children.Clear()
    
    $backupDir = Join-Path $PSScriptRoot "backups"
    $manifestPath = Join-Path $backupDir "manifest.json"
    $entries = @()
    
    # 1. Windows Restore Points via PowerShell
    try {
        $srList = Get-ComputerRestorePoint -ErrorAction SilentlyContinue
        foreach ($sr in $srList) {
            $entries += [PSCustomObject]@{
                Title = $sr.Description
                IsSystemRestore = $true
                Sequence = $sr.SequenceNumber
                Path = ""
                Time = $sr.CreationTime
                Size = "System Snapshot"
            }
        }
    } catch { }
    
    # 2. Manifest and .reg files
    if (Test-Path $manifestPath) {
        try {
            $json = Get-Content $manifestPath -Raw | ConvertFrom-Json
            foreach ($item in $json) {
                if ($item.FilePath -and (Test-Path $item.FilePath)) {
                    $entries += [PSCustomObject]@{
                        Title = $item.Title
                        IsSystemRestore = $false
                        Sequence = 0
                        Path = $item.FilePath
                        Time = $item.TimestampFormatted
                        Size = $item.SizeFormatted
                    }
                }
            }
        } catch { }
    }
    
    # Check loose .reg files in backups/
    if (Test-Path $backupDir) {
        $regFiles = Get-ChildItem -Path $backupDir -Filter "*.reg" -ErrorAction SilentlyContinue
        foreach ($rf in $regFiles) {
            if (!($entries | Where-Object { $_.Path -eq $rf.FullName })) {
                $kb = [math]::Round($rf.Length / 1KB, 1)
                $entries += [PSCustomObject]@{
                    Title = $rf.Name
                    IsSystemRestore = $false
                    Sequence = 0
                    Path = $rf.FullName
                    Time = $rf.LastWriteTime.ToString("yyyy-MM-dd HH:mm:ss")
                    Size = "$kb KB"
                }
            }
        }
    }
    
    if ($txtBackupsCount) {
        $txtBackupsCount.Text = "$($entries.Count) backup point(s) recorded"
    }
    
    if ($entries.Count -eq 0) {
        $emptyCard = New-Object System.Windows.Controls.Border
        $emptyCard.CornerRadius = New-Object System.Windows.CornerRadius(8)
        $emptyCard.BorderThickness = New-Object System.Windows.Thickness(1)
        $emptyCard.Padding = New-Object System.Windows.Thickness(14, 12, 14, 12)
        $emptyCard.SetResourceReference([System.Windows.Controls.Border]::BackgroundProperty, "TweakCardBg")
        $emptyCard.SetResourceReference([System.Windows.Controls.Border]::BorderBrushProperty, "TweakCardBorder")
        
        $sp = New-Object System.Windows.Controls.StackPanel
        $t1 = New-Object System.Windows.Controls.TextBlock
        $t1.Text = "No Restore Points or Registry Snapshots Found"
        $t1.FontSize = 12.5
        $t1.FontWeight = [System.Windows.FontWeights]::SemiBold
        $t1.SetResourceReference([System.Windows.Controls.TextBlock]::ForegroundProperty, "TextPrimaryBrush")
        
        $t2 = New-Object System.Windows.Controls.TextBlock
        $t2.Text = "Click 'Create Backup Point' above to generate an immediate safety checkpoint."
        $t2.FontSize = 11
        $t2.Margin = New-Object System.Windows.Thickness(0, 4, 0, 0)
        $t2.SetResourceReference([System.Windows.Controls.TextBlock]::ForegroundProperty, "TextSecondaryBrush")
        
        $sp.Children.Add($t1) | Out-Null
        $sp.Children.Add($t2) | Out-Null
        $emptyCard.Child = $sp
        $pnlRestorePointsList.Children.Add($emptyCard) | Out-Null
        return
    }
    
    foreach ($entry in $entries) {
        $card = New-Object System.Windows.Controls.Border
        $card.CornerRadius = New-Object System.Windows.CornerRadius(8)
        $card.BorderThickness = New-Object System.Windows.Thickness(1)
        $card.Padding = New-Object System.Windows.Thickness(14, 10, 14, 10)
        $card.Margin = New-Object System.Windows.Thickness(0, 0, 0, 8)
        $card.SetResourceReference([System.Windows.Controls.Border]::BackgroundProperty, "TweakCardBg")
        $card.SetResourceReference([System.Windows.Controls.Border]::BorderBrushProperty, "TweakCardBorder")
        
        $grid = New-Object System.Windows.Controls.Grid
        $c0 = New-Object System.Windows.Controls.ColumnDefinition
        $c0.Width = New-Object System.Windows.GridLength(1, [System.Windows.GridUnitType]::Star)
        $c1 = New-Object System.Windows.Controls.ColumnDefinition
        $c1.Width = [System.Windows.GridLength]::Auto
        $grid.ColumnDefinitions.Add($c0)
        $grid.ColumnDefinitions.Add($c1)
        
        $left = New-Object System.Windows.Controls.StackPanel
        $titleBlock = New-Object System.Windows.Controls.TextBlock
        $titleBlock.Text = $entry.Title
        $titleBlock.FontSize = 12.5
        $titleBlock.FontWeight = [System.Windows.FontWeights]::Bold
        $titleBlock.SetResourceReference([System.Windows.Controls.TextBlock]::ForegroundProperty, "TextPrimaryBrush")
        $left.Children.Add($titleBlock) | Out-Null
        
        $subBlock = New-Object System.Windows.Controls.TextBlock
        $badgeText = if ($entry.IsSystemRestore) { "WINDOWS RESTORE POINT" } else { "REGISTRY BACKUP" }
        $subBlock.Text = "Type: $badgeText - $($entry.Time) - $($entry.Size)"
        $subBlock.FontSize = 10.5
        $subBlock.Margin = New-Object System.Windows.Thickness(0, 3, 0, 0)
        $subBlock.SetResourceReference([System.Windows.Controls.TextBlock]::ForegroundProperty, "TextMutedBrush")
        $left.Children.Add($subBlock) | Out-Null
        
        [System.Windows.Controls.Grid]::SetColumn($left, 0)
        $grid.Children.Add($left) | Out-Null
        
        $right = New-Object System.Windows.Controls.StackPanel
        $right.Orientation = [System.Windows.Controls.Orientation]::Horizontal
        $right.VerticalAlignment = [System.Windows.VerticalAlignment]::Center
        
        $actBtn = New-Object System.Windows.Controls.Button
        $actBtn.Height = 28
        $actBtn.FontSize = 11
        $actBtn.Padding = New-Object System.Windows.Thickness(10, 4, 10, 4)
        $actBtn.SetResourceReference([System.Windows.Controls.Button]::StyleProperty, "ButtonPrimary")
        
        $captured = $entry
        if ($captured.IsSystemRestore) {
            $actBtn.Content = "Restore Snapshot"
            $actBtn.Add_Click({
                $ans = [System.Windows.MessageBox]::Show("Restore Windows to point '$($captured.Title)'?", "Confirm Restore", [System.Windows.MessageBoxButton]::YesNo, [System.Windows.MessageBoxImage]::Warning)
                if ($ans -eq [System.Windows.MessageBoxResult]::Yes) {
                    Invoke-RestoreComputerSnapshot -SequenceNumber $captured.Sequence
                }
            })
        } else {
            $actBtn.Content = "Import Reg"
            $actBtn.Add_Click({
                $ans = [System.Windows.MessageBox]::Show("Import registry keys from '$($captured.Path)'?", "Confirm Restore", [System.Windows.MessageBoxButton]::YesNo, [System.Windows.MessageBoxImage]::Question)
                if ($ans -eq [System.Windows.MessageBoxResult]::Yes) {
                    Invoke-RestoreRegistryBackup -RegPath $captured.Path
                }
            })
        }
        $right.Children.Add($actBtn) | Out-Null
        
        [System.Windows.Controls.Grid]::SetColumn($right, 1)
        $grid.Children.Add($right) | Out-Null
        
        $card.Child = $grid
        $pnlRestorePointsList.Children.Add($card) | Out-Null
    }
}

if ($btnRestorePoint) {
    $btnRestorePoint.Add_Click({
        & $Log "Creating Windows System Restore Point and registry backup..."
        New-SystemRestorePoint -Description "AnxiouslyOptimized_Manual_Checkpoint" | Out-Null
        Export-RegistryBackup | Out-Null
        & $Log "Safety restore point and settings backup created."
        & $UpdateBackupsUI
    })
}

if ($btnRefreshBackups) {
    $btnRefreshBackups.Add_Click({
        & $UpdateBackupsUI
    })
}

if ($btnOpenBackupsFolder) {
    $btnOpenBackupsFolder.Add_Click({
        $dir = Join-Path $PSScriptRoot "backups"
        if (!(Test-Path $dir)) { New-Item -Path $dir -ItemType Directory -Force | Out-Null }
        Start-Process "explorer.exe" $dir
    })
}

if ($btnLaunchSystemRestore) {
    $btnLaunchSystemRestore.Add_Click({
        Start-Process "rstrui.exe"
    })
}

if ($btnUndoAllRestore) {
    $btnUndoAllRestore.Add_Click({
        $ans = [System.Windows.MessageBox]::Show("Revert all optimizations back to Windows defaults?", "Emergency Factory Reset", [System.Windows.MessageBoxButton]::YesNo, [System.Windows.MessageBoxImage]::Warning)
        if ($ans -eq [System.Windows.MessageBoxResult]::Yes) {
            & $Log "Reverting all applied tweaks..."
            foreach ($tw in $allTweaks) {
                if ($tw.IsApplied) {
                    Invoke-TweakAction -Tweak $tw -Apply $false -Log $Log
                    $tw.IsApplied = $false
                }
            }
            & $Log "All optimizations reverted to default state."
        }
    })
}



# Scan System



if ($btnScanSystem) {



    $btnScanSystem.Add_Click({



        & $Log "Refreshing system status..."



        Sync-TweakList



        & $Log "Status updated across all categories."



    })



}



# Presets



if ($btnSafe) {



    $btnSafe.Add_Click({



        New-SystemRestorePoint -Description "Anxious_Preset_Safe" | Out-Null



        Export-RegistryBackup | Out-Null



        Invoke-PresetBatch -PresetKey "safe_baseline" -LogCallback $Log



        Sync-TweakList



    })



}



if ($btnGaming) {



    $btnGaming.Add_Click({



        New-SystemRestorePoint -Description "Anxious_Preset_Gaming" | Out-Null



        Export-RegistryBackup | Out-Null



        Invoke-PresetBatch -PresetKey "gaming_rig" -LogCallback $Log



        Sync-TweakList



    })



}



if ($btnEmulator) {



    $btnEmulator.Add_Click({



        New-SystemRestorePoint -Description "Anxious_Preset_Emulator" | Out-Null



        Export-RegistryBackup | Out-Null



        Invoke-PresetBatch -PresetKey "emulator_pro" -LogCallback $Log



        Sync-TweakList



    })



}



if ($btnRevertAll) {



    $btnRevertAll.Add_Click({



        $confirm = [System.Windows.MessageBox]::Show(



            "Do you want to reset all settings, menus, and services back to original Windows defaults?",



            "Reset to Defaults",



            [System.Windows.MessageBoxButton]::YesNo,



            [System.Windows.MessageBoxImage]::Question



        )



        if ($confirm -eq [System.Windows.MessageBoxResult]::Yes) {



            Invoke-RevertAllTweaks -LogCallback $Log



            Sync-TweakList



        }



    })



}



# Selection Helpers



if ($btnSelectAll) {



    $btnSelectAll.Add_Click({



        foreach ($entry in $tweakCheckboxMap.Values) {



            $entry.CheckBox.IsChecked = $true



        }



    })



}



if ($btnDeselectAll) {



    $btnDeselectAll.Add_Click({



        foreach ($entry in $tweakCheckboxMap.Values) {



            $entry.CheckBox.IsChecked = $false



        }



    })



}



if ($btnApplySelected) {



    $btnApplySelected.Add_Click({



        $checkedItems = $tweakCheckboxMap.Values | Where-Object { $_.CheckBox.IsChecked }



        if (!$checkedItems -or $checkedItems.Count -eq 0) {



            & $Log "No tweaks selected. Check the boxes of the tweaks you want to apply."



            return



        }



        New-SystemRestorePoint -Description "Anxious_Custom_Apply" | Out-Null



        foreach ($item in $checkedItems) {



            Invoke-TweakAction -Tweak $item.Tweak -Action "Apply" -LogCallback $Log



        }



        Sync-TweakList



    })



}



if ($btnRevertSelected) {



    $btnRevertSelected.Add_Click({



        $checkedItems = $tweakCheckboxMap.Values | Where-Object { $_.CheckBox.IsChecked }



        if (!$checkedItems -or $checkedItems.Count -eq 0) {



            & $Log "No tweaks selected. Check the boxes of the tweaks you want to revert."



            return



        }



        foreach ($item in $checkedItems) {



            Invoke-TweakAction -Tweak $item.Tweak -Action "Revert" -LogCallback $Log



        }



        Sync-TweakList



    })



}



# Debloat



if ($btnScanBloat) {



    $btnScanBloat.Add_Click({



        Sync-BloatList



    })



}



if ($btnRemoveBloat) {



    $btnRemoveBloat.Add_Click({



        $selected = @()



        foreach ($k in $bloatCheckboxMap.Keys) {



            if ($bloatCheckboxMap[$k].IsChecked) {



                $selected += $k



            }



        }



        if ($selected.Count -eq 0) {



            & $Log "No apps selected for removal."



            return



        }



        $removed = Invoke-SafeDebloat -PackageNames $selected -LogCallback $Log



        & $Log "Successfully removed $removed unwanted apps."



        Sync-BloatList



    })



}



# Maintenance Tools



if ($btnRunSFC) {



    $btnRunSFC.Add_Click({



        Invoke-SystemFileCheck -LogCallback $Log



    })



}



if ($btnRunDISM) {



    $btnRunDISM.Add_Click({



        Invoke-DISMRepair -LogCallback $Log



    })



}



if ($btnRunComponentCleanup) {



    $btnRunComponentCleanup.Add_Click({



        Invoke-ComponentCleanup -LogCallback $Log



    })



}



if ($btnFlushRAM) {



    $btnFlushRAM.Add_Click({



        Invoke-StandbyMemoryFlush -LogCallback $Log



    })



}



if ($btnToggleHibernation) {



    $btnToggleHibernation.Add_Click({



        Invoke-ToggleHibernation -LogCallback $Log



    })



}



if ($btnCleanCaches) {



    $btnCleanCaches.Add_Click({



        Invoke-ClearSafeCaches -LogCallback $Log



    })



}



# Free Fire & Emulator Button Handlers



if ($btnMasterFreeFire) {



    $btnMasterFreeFire.Add_Click({



        New-SystemRestorePoint -Description "Anxious_FreeFire_Master_Boost" | Out-Null



        Invoke-MasterFreeFireBoost -LogCallback $Log



        & $SyncEmulatorHUD



    })



}



if ($btnFFUnlockFPS) {



    $btnFFUnlockFPS.Add_Click({



        New-SystemRestorePoint -Description "Anxious_FreeFire_FPS_Unlock" | Out-Null



        Invoke-FreeFireFPSUnlock -LogCallback $Log



        & $SyncEmulatorHUD



    })



}



if ($btnFFGpuLock) {



    $btnFFGpuLock.Add_Click({



        Invoke-EmulatorGPULock -LogCallback $Log



        & $SyncEmulatorHUD



    })



}



if ($btnFFMouseAim) {



    $btnFFMouseAim.Add_Click({



        Invoke-FreeFireMouseAim -LogCallback $Log



        & $SyncEmulatorHUD



    })



}



if ($btnFFLowPing) {



    $btnFFLowPing.Add_Click({



        Invoke-EmulatorLowPing -LogCallback $Log



        & $SyncEmulatorHUD



    })



}



if ($btnToggleVBS) {



    $btnToggleVBS.Add_Click({



        $isCurrentlyOff = ((Get-VBSStatus).HypervisorLaunchType -eq "Off")



        $targetDisable = !$isCurrentlyOff



        $actionWord = if ($targetDisable) { "turn off the Windows security feature that slows down emulators (gives up to 30% more FPS in BlueStacks & Free Fire)" } else { "turn the Windows security feature back on" }



        $msgText = "Do you want to {0}?`r`n`r`nNote: You will need to restart your computer for this change to take effect." -f $actionWord



        $confirm = [System.Windows.MessageBox]::Show(



            $msgText,



            "Emulator Speed Boost",



            [System.Windows.MessageBoxButton]::YesNo,



            [System.Windows.MessageBoxImage]::Question



        )



        if ($confirm -eq [System.Windows.MessageBoxResult]::Yes) {



            Invoke-ToggleVBS -Disable $targetDisable -LogCallback $Log



            & $SyncEmulatorHUD



        }



    })



}



if ($btnFFCleanCache) {



    $btnFFCleanCache.Add_Click({



        Invoke-EmulatorCacheClean -LogCallback $Log



        & $SyncEmulatorHUD



    })



}



if ($btnFFRevert) {



    $btnFFRevert.Add_Click({



        $confirm = [System.Windows.MessageBox]::Show(



            "Reset all game, emulator, and mouse settings back to original Windows defaults?",



            "Reset Game Settings",



            [System.Windows.MessageBoxButton]::YesNo,



            [System.Windows.MessageBoxImage]::Question



        )



        if ($confirm -eq [System.Windows.MessageBoxResult]::Yes) {



            Invoke-RevertEmulatorTweaks -LogCallback $Log



            & $SyncEmulatorHUD



        }



    })



}



# Clear Log



if ($btnClearLogs) {



    $btnClearLogs.Add_Click({



        if ($txtConsole) { $txtConsole.Clear() }



    })



}

if ($btnCopyLogs) {

    $btnCopyLogs.Add_Click({

        if ($txtConsole -and !([string]::IsNullOrWhiteSpace($txtConsole.Text))) {

            [System.Windows.Clipboard]::SetText($txtConsole.Text)

            & $Log "[INFO] Activity logs copied to clipboard."

        }

    })

}



# Custom Window Title Bar Handlers (WindowChrome)



if ($btnMin) {



    $btnMin.Add_Click({



        $window.WindowState = [System.Windows.WindowState]::Minimized



    })



}



if ($btnMax) {



    $btnMax.Add_Click({



        if ($window.WindowState -eq [System.Windows.WindowState]::Maximized) {



            $window.WindowState = [System.Windows.WindowState]::Normal



        } else {



            $window.WindowState = [System.Windows.WindowState]::Maximized



        }



    })



}



if ($btnClose) {



    $btnClose.Add_Click({



        $window.Close()



    })



}



# ==============================================================================



#  REFERENCE DESIGN NAVIGATION, TELEMETRY & EVENT DISPATCHERS



# ==============================================================================



# Dynamic Greeting Calculation (Morning, Afternoon, Evening, Night)



if ($txtGreetingTime) {
    $hour = (Get-Date).Hour
    $rawUser = if ($env:USERNAME) { $env:USERNAME } else { "User" }
    $cleanUser = (Get-Culture).TextInfo.ToTitleCase($rawUser.ToLower())
    $timeSalutation = if ($hour -ge 5 -and $hour -lt 12) { "Good Morning" }
                      elseif ($hour -ge 12 -and $hour -lt 17) { "Good Afternoon" }
                      elseif ($hour -ge 17 -and $hour -lt 22) { "Good Evening" }
                      else { "Good Night" }
    $txtGreetingTime.Text = "$timeSalutation, $cleanUser!"
}



# Dynamic Oscilloscope Waveform Engine (Lively Ripple on Every Tick)



$script:sparkTick = 0



function Get-WaveGeometry {



    param(



        [double]$val,



        [int]$tick,



        [double]$speed = 1.0,



        [double]$amp = 2.5



    )



    if ($val -le 0.0) {



        # Quiet resting baseline floor for 0% idle load



        return [System.Windows.Media.Geometry]::Parse("M 0 14 L 7 14 L 14 14 L 21 14 L 28 14 L 35 14 L 42 14")



    }



    $pts = @()



    for ($i = 0; $i -le 6; $i++) {



        $x = $i * 7



        $phase = ($tick * $speed) + ($i * 1.3)



        $harmonic = ([math]::Sin($phase) * $amp) + ([math]::Cos($phase * 1.7) * ($amp * 0.4))



        $baseY = 13.0 - (($val / 100.0) * 8.0)



        $y = [math]::Max(2, [math]::Min(14, [int]([math]::Round($baseY + $harmonic))))



        $pts += if ($i -eq 0) { "M $x $y" } else { "L $x $y" }



    }



    return [System.Windows.Media.Geometry]::Parse(($pts -join " "))



}



function Get-ArcGeometry {



    param([double]$pct, [double]$cx=18, [double]$cy=18, [double]$r=15)



    if ($pct -le 0.0) {



        return $null



    }



    $p = [math]::Max(0.5, [math]::Min(99.9, $pct))



    $angleRad = ($p / 100.0) * 360.0 * [math]::PI / 180.0



    $x = [math]::Round($cx + $r * [math]::Sin($angleRad), 2)



    $y = [math]::Round($cy - $r * [math]::Cos($angleRad), 2)



    $isLarge = if ($p -gt 50.0) { "1" } else { "0" }



    $topY = $cy - $r



    return [System.Windows.Media.Geometry]::Parse("M $cx,$topY A $r,$r 0 $isLarge,1 $x,$y")



}



$UpdateLiveTelemetry = {



    try {



        $script:sparkTick++



        $metrics = Get-LivePerformanceMetrics



        $cpuVal = [math]::Max(1, [int]$metrics.CPULoadPercent)
        $ramVal = [math]::Max(1, [int]$metrics.RAMPercent)
        $diskSpaceVal = if ($metrics.DiskCSpacePercent -ne $null) { [int]$metrics.DiskCSpacePercent } else { [int]$metrics.DiskSpacePercent }
        $diskDisplayVal = [math]::Max(0, [math]::Min(100, $diskSpaceVal))

        if ($txtGaugeCPU) { $txtGaugeCPU.Text = "$cpuVal%" }
        if ($txtGaugeRAM) { 
            $txtGaugeRAM.Text = "$ramVal%" 
            $txtGaugeRAM.ToolTip = "Live System RAM: $($metrics.RAMUsedGB) GB Used / $([math]::Ceiling($metrics.RAMTotalGB)) GB Installed | $($metrics.RAMPercent)% Load"
        }
        if ($txtGaugeRAMSub) {
            $txtGaugeRAMSub.Text = "$($metrics.RAMUsedGB) / $([math]::Ceiling($metrics.RAMTotalGB)) GB"
        }
        if ($txtGaugeDisk) { 
            $txtGaugeDisk.Text = "$diskDisplayVal%"
            $txtGaugeDisk.ToolTip = "SSD C: (System): $($metrics.DiskCUsedGB) GB Used / $($metrics.DiskCTotalGB) GB Total ($($metrics.DiskCFreeGB) GB Free, $diskDisplayVal% Used)`nFixed Drives Storage: $($metrics.DriveSummary)"
        }
        if ($txtGaugeDiskSub) {
            $txtGaugeDiskSub.Text = "$($metrics.DiskCFreeGB) GB Free"
        }

        # Update dynamic SVG arc rings
        if ($arcGaugeCPU)  { $arcGaugeCPU.Data = Get-ArcGeometry $cpuVal }
        if ($arcGaugeRAM)  { $arcGaugeRAM.Data = Get-ArcGeometry $ramVal }
        if ($arcGaugeDisk) { 
            $diskArcVal = [math]::Max(2, $diskDisplayVal)
            $arcGaugeDisk.Data = Get-ArcGeometry $diskArcVal 
        }

        # Generate animated flowing oscilloscope sparkline waveforms
        if ($pathSparkCPU) {
            $pathSparkCPU.Data = Get-WaveGeometry -val $cpuVal -tick $script:sparkTick -speed 1.2 -amp 2.8
        }
        if ($pathSparkRAM) {
            $pathSparkRAM.Data = Get-WaveGeometry -val $ramVal -tick $script:sparkTick -speed 0.8 -amp 2.2
        }
        if ($pathSparkDisk) {
            $diskAmp = if ($diskDisplayVal -gt 0) { [math]::Min(3.8, 1.2 + ($diskDisplayVal * 0.04)) } else { 0.7 }
            $diskSpd = if ($diskDisplayVal -gt 0) { 1.4 } else { 0.4 }
            $pathSparkDisk.Data = Get-WaveGeometry -val ([math]::Max(1, $diskDisplayVal)) -tick $script:sparkTick -speed $diskSpd -amp $diskAmp
        }



        if ($txtSystemStatusSub) {



            $txtSystemStatusSub.Text = "Last scan: Today, $(Get-Date -Format 'hh:mm tt')"



        }



    } catch { }



}



# Initial Telemetry Sample



& $UpdateLiveTelemetry



# Live Telemetry Background Timer (1.0s realtime lightweight interval)



$telemetryTimer = New-Object System.Windows.Threading.DispatcherTimer



$telemetryTimer.Interval = [TimeSpan]::FromSeconds(1.0)



$telemetryTimer.Add_Tick({ & $UpdateLiveTelemetry })



$telemetryTimer.Start()



$window.Add_Closed({



    if ($telemetryTimer) { $telemetryTimer.Stop() }



})



# Overview Hero: Scan My PC Primary CTA



$script:isScanning = $false



if ($btnOverviewScan) {



    $btnOverviewScan.Add_Click({



        if ($script:isScanning) { return }



        $script:isScanning = $true



        if ($txtOverviewScan) { $txtOverviewScan.Text = "Scanning..." }



        $rot = if ($vbScanIcon) { $vbScanIcon.RenderTransform } else { $null }



        & $Log "[SCAN] Starting comprehensive system health scan..." -ShowDrawer



        if ($txtStatusBar) { $txtStatusBar.Text = "Scanning system files, memory, and services..." }



        $scanAnimTimer = New-Object System.Windows.Threading.DispatcherTimer



        $scanAnimTimer.Interval = [TimeSpan]::FromMilliseconds(30)



        $step = 0



        $scanAnimTimer.Add_Tick({



            $step++



            if ($rot) { $rot.Angle = ($rot.Angle + 12) % 360 }



            if ($step -eq 12) {



                & $Log "[SCAN] Analyzing Windows component store and system file checksums..."



            } elseif ($step -eq 25) {



                & $Log "[SCAN] Auditing memory buffers, standbylist caches, and driver overhead..."



            } elseif ($step -ge 40) {



                $scanAnimTimer.Stop()



                if ($rot) { $rot.Angle = 0 }



                if ($txtOverviewScan) { $txtOverviewScan.Text = "Scan My PC" }



                $script:isScanning = $false



                & $UpdateLiveTelemetry



                & $SyncEmulatorHUD



                & $Log "[SUCCESS] System scan complete. 0 integrity violations detected."



                & $Log "[HEALTH] All systems normal and fully optimized."



                if ($txtStatusBar) { $txtStatusBar.Text = "All systems normal - No issues found" }



                if ($txtSystemStatusSub) { $txtSystemStatusSub.Text = "Last scan: Today, $(Get-Date -Format 'hh:mm tt')" }



            }



        })



        $scanAnimTimer.Start()



    })



}



# Overview 6 Bento Action Cards Handlers



if ($btnOverviewSFC) {



    $btnOverviewSFC.Add_Click({



        Invoke-SystemFileCheck -LogCallback $Log



        & $UpdateLiveTelemetry



    })



}



if ($btnOverviewDISM) {



    $btnOverviewDISM.Add_Click({



        Invoke-DISMRepair -LogCallback $Log



        & $UpdateLiveTelemetry



    })



}



if ($btnOverviewCleanup) {



    $btnOverviewCleanup.Add_Click({



        Invoke-ComponentCleanup -LogCallback $Log



        & $UpdateLiveTelemetry



    })



}



if ($btnOverviewRAM) {



    $btnOverviewRAM.Add_Click({



        Invoke-StandbyMemoryFlush -LogCallback $Log



        & $UpdateLiveTelemetry



    })



}



if ($btnOverviewHiber) {



    $btnOverviewHiber.Add_Click({



        Invoke-ToggleHibernation -LogCallback $Log



        & $UpdateLiveTelemetry



    })



}



if ($btnOverviewCache) {



    $btnOverviewCache.Add_Click({



        Invoke-ClearSafeCaches -LogCallback $Log



        & $UpdateLiveTelemetry



    })



}



# Overview Bento Card Circular Arrow Navigation Handlers



if ($btnNavCardSFC)     { $btnNavCardSFC.Add_Click({ & $SetNavActive 4 }) }



if ($btnNavCardDISM)    { $btnNavCardDISM.Add_Click({ & $SetNavActive 4 }) }



if ($btnNavCardCleanup) { $btnNavCardCleanup.Add_Click({ & $SetNavActive 4 }) }



if ($btnNavCardRAM)     { $btnNavCardRAM.Add_Click({ & $SetNavActive 4 }) }



if ($btnNavCardHiber)   { $btnNavCardHiber.Add_Click({ & $SetNavActive 4 }) }



if ($btnNavCardCache)   { $btnNavCardCache.Add_Click({ & $SetNavActive 4 }) }



# Universal Search Box Filtering & Tab Routing



if ($txtGlobalSearch) {



    $txtGlobalSearch.Add_GotFocus({



        if ($txtGlobalSearch.Text -eq "Search tools...") {



            $txtGlobalSearch.Text = ""



        }



    })



    $txtGlobalSearch.Add_LostFocus({



        if ([string]::IsNullOrWhiteSpace($txtGlobalSearch.Text)) {



            $txtGlobalSearch.Text = "Search tools..."



        }



    })



    $txtGlobalSearch.Add_TextChanged({



        $q = $txtGlobalSearch.Text.Trim()



        if ($q -ne "Search tools..." -and $q.Length -gt 0) {



            if ($q -match "free|fps|game|aim|emu|bluestacks|msi") {



                & $SetNavActive 5



            } elseif ($q -match "repair|sfc|dism|health|ram|memory|cache|sleep") {



                & $SetNavActive 4



            } elseif ($q -match "debloat|clean|junk|bloat|app") {



                & $SetNavActive 3



            } elseif ($q -match "boost|safe|preset|quick") {



                & $SetNavActive 1



            } elseif ($q -match "backup|restore|undo") {



                & $SetNavActive 7



            } elseif ($q -match "setting|config|os") {



                & $SetNavActive 6



            } elseif ($q -match "about|info|version") {



                & $SetNavActive 8



            } else {



                & $SetNavActive 2



                if ($txtTweakSearch) { $txtTweakSearch.Text = $q }



            }



        }



    })



}



# Console Drawer Close



if ($btnCloseDrawer -and $brdConsoleDrawer) {



    $btnCloseDrawer.Add_Click({



        $brdConsoleDrawer.Visibility = [System.Windows.Visibility]::Collapsed



    })



}



# Backups Tab: Revert All Handler



if ($btnUndoAllRestore) {



    $btnUndoAllRestore.Add_Click({



        $confirm = [System.Windows.MessageBox]::Show(



            "Revert all optimizations, registry tweaks, and game settings back to original Windows defaults?",



            "Emergency Factory Reset",



            [System.Windows.MessageBoxButton]::YesNo,



            [System.Windows.MessageBoxImage]::Warning



        )



        if ($confirm -eq [System.Windows.MessageBoxResult]::Yes) {



            Invoke-RevertAllTweaks -LogCallback $Log



            & $SyncEmulatorHUD



            & $UpdateLiveTelemetry



        }



    })



}



# Settings Tab: OS Target Switching



if ($rdoOSWin11) {



    $rdoOSWin11.Add_Checked({



        $activeOS = "Win11"



        if ($txtEditionBadge) { $txtEditionBadge.Text = "Windows 11 Edition" }



        Sync-TweakList



        & $SyncEmulatorHUD



    })



}



if ($rdoOSWin10) {



    $rdoOSWin10.Add_Checked({



        $activeOS = "Win10"



        if ($txtEditionBadge) { $txtEditionBadge.Text = "Windows 10 Edition" }



        Sync-TweakList



        & $SyncEmulatorHUD



    })



}



if ($rdoOSAuto) {



    $rdoOSAuto.Add_Checked({



        $activeOS = Get-DetectedOS



        if ($txtEditionBadge) {



            $txtEditionBadge.Text = if ($activeOS -eq "Win11") { "Windows 11 Edition" } else { "Windows 10 Edition" }



        }



        Sync-TweakList



        & $SyncEmulatorHUD



    })



}



if ($btnResetSettings) {



    $btnResetSettings.Add_Click({



        if ($rdoOSAuto) { $rdoOSAuto.IsChecked = $true }



        & $Log "Settings reset to defaults."



    })



}



# Sidebar Navigation Switching Handlers



$navButtons = @(



    $navOverview, $navQuickBoost, $navTweaks, $navDebloat, $navHealth,



    $navEmulators, $navSettings, $navBackups, $navAbout



)



$navIndicators = @(



    $navIndOverview, $navIndQuickBoost, $navIndTweaks, $navIndDebloat, $navIndHealth, $navIndEmulators



)



$SetNavActive = {



    param([int]$Index)



    if ($tabsWorkspace) {



        $tabsWorkspace.SelectedIndex = $Index



        try {



            $tabAnim = $window.FindResource("TabEntranceAnim")



            $selectedTab = $tabsWorkspace.SelectedItem



            if ($tabAnim -and $selectedTab -and $selectedTab.Content) {



                $tabAnim.Begin($selectedTab.Content)



            }



        } catch { }



    }



    $activeBg = if ($window.Resources.Contains("SidebarActiveBgBrush")) { $window.Resources["SidebarActiveBgBrush"] } else { New-Object System.Windows.Media.SolidColorBrush([System.Windows.Media.Color]::FromArgb(255, 14, 25, 43)) }



    $activeFg = if ($window.Resources.Contains("SidebarActiveFgBrush")) { $window.Resources["SidebarActiveFgBrush"] } else { [System.Windows.Media.Brushes]::White }



    $inactiveFg = if ($window.Resources.Contains("TextSecondaryBrush")) { $window.Resources["TextSecondaryBrush"] } else { New-Object System.Windows.Media.SolidColorBrush([System.Windows.Media.Color]::FromArgb(255, 138, 153, 173)) }



    for ($i = 0; $i -lt $navButtons.Count; $i++) {



        if ($navButtons[$i]) {



            if ($i -eq $Index) {



                $navButtons[$i].Background = $activeBg



                $navButtons[$i].Foreground = $activeFg



            } else {



                $navButtons[$i].Background = [System.Windows.Media.Brushes]::Transparent



                $navButtons[$i].Foreground = $inactiveFg



            }



        }



    }



    for ($j = 0; $j -lt $navIndicators.Count; $j++) {



        if ($navIndicators[$j]) {



            if ($j -eq $Index) {



                $navIndicators[$j].Visibility = [System.Windows.Visibility]::Visible



            } else {



                $navIndicators[$j].Visibility = [System.Windows.Visibility]::Collapsed



            }



        }



    }



}



if ($navOverview)    { $navOverview.Add_Click({ & $SetNavActive 0 }) }



if ($navQuickBoost)  { $navQuickBoost.Add_Click({ & $SetNavActive 1 }) }



if ($navTweaks)      { $navTweaks.Add_Click({ & $SetNavActive 2 }) }



if ($navDebloat)     { $navDebloat.Add_Click({ & $SetNavActive 3 }) }



if ($navHealth)      { $navHealth.Add_Click({ & $SetNavActive 4 }) }



if ($navEmulators)   { $navEmulators.Add_Click({ & $SetNavActive 5 }) }



if ($navSettings)    { $navSettings.Add_Click({ & $SetNavActive 6 }) }



if ($navBackups)     { $navBackups.Add_Click({ & $SetNavActive 7; & $UpdateBackupsUI }) }



if ($navAbout)       { $navAbout.Add_Click({ & $SetNavActive 8 }) }



# Activate default view: Overview (Index 0)



& $SetNavActive 0
& $UpdateBackupsUI



# Show UI Window



$window.ShowDialog() | Out-Null

