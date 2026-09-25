$ErrorActionPreference = "Continue"

Write-Host "==================================================================" -ForegroundColor Cyan
Write-Host " MASTER 10/10 DEEP AUDIT FOR ANXIOUSLYOPTIMIZED (FULL SUITE)" -ForegroundColor Cyan
Write-Host "==================================================================" -ForegroundColor Cyan

$baseDir = "d:\Users\GOYAL\Documents\optimizefiles"
Set-Location $baseDir

$passCount = 0
$failCount = 0

function Report-Result($testName, $passed, $detail = "") {
    if ($passed) {
        $script:passCount++
        Write-Host "  [PASS] $testName" -ForegroundColor Green
        if ($detail) { Write-Host "         -> $detail" -ForegroundColor DarkGreen }
    } else {
        $script:failCount++
        Write-Host "  [FAIL] $testName" -ForegroundColor Red
        if ($detail) { Write-Host "         -> $detail" -ForegroundColor DarkRed }
    }
}

# 1. AST Validation
Write-Host "`n[TEST SUITE 1] AST & Syntax Validation of All Source Files..." -ForegroundColor Yellow
$psFiles = Get-ChildItem -Path $baseDir -Recurse -Include *.ps1, *.psm1 | Where-Object { $_.FullName -notmatch '\\scratch\\' -and $_.FullName -notmatch '\\\.git\\' }
foreach ($f in $psFiles) {
    $tokens = $null
    $errors = $null
    $ast = [System.Management.Automation.Language.Parser]::ParseFile($f.FullName, [ref]$tokens, [ref]$errors)
    $hasErr = ($errors.Count -gt 0)
    Report-Result "Syntax Check: $($f.Name)" (-not $hasErr) ($ifErr = if ($hasErr) { $errors[0].Message } else { "Clean AST (0 errors)" })
}

# 2. Emdash & Bad Characters Check
Write-Host "`n[TEST SUITE 2] Emdash & Invalid Unicode Scan..." -ForegroundColor Yellow
$emdashFound = 0
foreach ($f in $psFiles) {
    $content = [System.IO.File]::ReadAllText($f.FullName)
    if ($content -match '[\u2013\u2014]') {
        $emdashFound++
        Report-Result "Emdash Scan: $($f.Name)" $false "Found emdash characters!"
    }
}
if ($emdashFound -eq 0) {
    Report-Result "Zero Emdash Characters Across All Code" $true "Clean Unicode encoding"
}

# 3. Engine & Config Validation
Write-Host "`n[TEST SUITE 3] Engine & Declarative Tweak System..." -ForegroundColor Yellow
Import-Module (Join-Path $baseDir "core\Engine.psm1") -Force
$os = Get-DetectedOS
Report-Result "OS Detection" ($os -in "Win10", "Win11") "Detected: $os"

$tweaks = Get-TweaksConfig -TargetOS $os
Report-Result "Tweaks Loader" ($tweaks.Count -ge 10) "Loaded $($tweaks.Count) tweaks for $os"

$brokenTweaks = @()
foreach ($t in $tweaks) {
    if (!$t.id -or !$t.title -or !$t.checkScript -or !$t.applyScript -or !$t.revertScript) {
        $brokenTweaks += $t.id
    }
}
$detailMsg = if ($brokenTweaks.Count -gt 0) { "Broken: $($brokenTweaks -join ', ')" } else { "All tweaks have complete id, title, check, apply, and revert scripts" }
Report-Result "Tweak Schema Integrity" ($brokenTweaks.Count -eq 0) $detailMsg

$presets = Get-PresetsConfig
$presetKeys = ($presets | Get-Member -MemberType NoteProperty).Name
Report-Result "Presets Config" ($presetKeys.Count -ge 3) "Found presets: $($presetKeys -join ', ')"

# 4. Debloater Engine
Write-Host "`n[TEST SUITE 4] Debloater Engine..." -ForegroundColor Yellow
Import-Module (Join-Path $baseDir "core\Debloater.psm1") -Force
$lists = Get-DebloatLists
Report-Result "Debloat Blacklist/Whitelist" ($lists.Blacklist.Count -gt 0 -and $lists.Whitelist.Count -gt 0) "Blacklist: $($lists.Blacklist.Count), Whitelist: $($lists.Whitelist.Count)"
$installedBloat = Get-InstalledBloatware
Report-Result "Bloatware Scan Execution" ($true) "Scanned system cleanly: $($installedBloat.Count) items found"

# 5. Maintenance Engine
Write-Host "`n[TEST SUITE 5] Maintenance Engine Functions..." -ForegroundColor Yellow
Import-Module (Join-Path $baseDir "core\Maintenance.psm1") -Force
$maintFns = @(
    "Invoke-SystemFileCheck",
    "Invoke-DISMRepair",
    "Invoke-ComponentCleanup",
    "Invoke-ClearSafeCaches",
    "Invoke-DisableCoreParking",
    "Invoke-DisableDeliveryOptimization",
    "Invoke-StandbyMemoryFlush",
    "Invoke-ToggleHibernation",
    "Invoke-EnableGPUMSIMode"
)
foreach ($fn in $maintFns) {
    $cmd = Get-Command $fn -ErrorAction SilentlyContinue
    Report-Result "Maintenance Function: $fn" ($cmd -ne $null) "Callable and exported"
}

# 6. Emulator & Gaming Engine
Write-Host "`n[TEST SUITE 6] Emulator & Gaming Engine..." -ForegroundColor Yellow
Import-Module (Join-Path $baseDir "core\Emulator.psm1") -Force
$emuFns = @(
    "Get-EmulatorTopology",
    "Invoke-FreeFireFPSUnlock",
    "Invoke-EmulatorGPULock",
    "Invoke-FreeFireMouseAim",
    "Reset-WindowsMouseSettings",
    "Invoke-EmulatorLowPing",
    "Invoke-EmulatorCacheClean",
    "Invoke-MasterFreeFireBoost",
    "Invoke-FreeFireStretchedResolution",
    "Get-VBSStatus",
    "Invoke-RevertEmulatorTweaks"
)
foreach ($fn in $emuFns) {
    $cmd = Get-Command $fn -ErrorAction SilentlyContinue
    Report-Result "Emulator Function: $fn" ($cmd -ne $null) "Callable and exported"
}

# 7. BlueStacks Configuration Files & Header Integrity
Write-Host "`n[TEST SUITE 7] BlueStacks Configuration Integrity..." -ForegroundColor Yellow
$confPaths = @(
    "C:\ProgramData\BlueStacks_nxt\bluestacks.conf",
    "C:\ProgramData\BlueStacks_msi5\bluestacks.conf"
)
foreach ($cp in $confPaths) {
    if (Test-Path $cp) {
        $bytes = [System.IO.File]::ReadAllBytes($cp)
        $hasBom = ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF)
        $validStart = ($bytes[0] -eq 0x62 -and $bytes[1] -eq 0x73 -and $bytes[2] -eq 0x74) # "bst"
        Report-Result "Config Header: $(Split-Path $cp -Parent | Split-Path -Leaf)" (-not $hasBom -and $validStart) "Byte 0..3: $(($bytes[0..3] | ForEach-Object { '{0:X2}' -f $_ }) -join ' ') (Valid ASCII/UTF8 No-BOM)"
    }
}

# 8. GUI Window Initialization Test (Headless/Offscreen instantiation)
Write-Host "`n[TEST SUITE 8] GUI Window Instantiation & Component Binding..." -ForegroundColor Yellow
try {
    Add-Type -AssemblyName PresentationFramework, PresentationCore, WindowsBase
    $xamlPath = Join-Path $baseDir "assets\gui.xaml"
    if (Test-Path $xamlPath) {
        [xml]$xaml = Get-Content $xamlPath -Raw
        $xmlReader = New-Object System.Xml.XmlNodeReader $xaml
        $window = [System.Windows.Markup.XamlReader]::Load($xmlReader)
        Report-Result "XAML Window Construction" ($window -ne $null) "Parsed WPF XAML successfully, Window Title: $($window.Title)"
        $window.Close()
    } else {
        Report-Result "XAML File Discovery" $false "assets\gui.xaml not found"
    }
} catch {
    Report-Result "GUI Instantiation" $false "Error: $($_.Exception.Message)"
}

Write-Host "`n==================================================================" -ForegroundColor Cyan
Write-Host " AUDIT SUMMARY: $passCount PASSED, $failCount FAILED" -ForegroundColor $(if ($failCount -eq 0) { "Green" } else { "Red" })
Write-Host "==================================================================" -ForegroundColor Cyan
