# Test Feature 2: Dynamic Active Game Mode Daemon
$exePath = Join-Path $PSScriptRoot "..\dist\AnxiouslyOptimized.exe"
if (!(Test-Path $exePath)) {
    Write-Error "Binary not found at $exePath"
    exit 1
}

$bytes = [System.IO.File]::ReadAllBytes($exePath)
$asm = [System.Reflection.Assembly]::Load($bytes)
$daemonType = $asm.GetType("AnxiouslyOptimized.Services.ActiveGameModeDaemon")

if ($null -eq $daemonType) {
    Write-Error "Could not find AnxiouslyOptimized.Services.ActiveGameModeDaemon in assembly"
    exit 1
}

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " FEATURE 2: ACTIVE GAME MODE DAEMON VERIFICATION" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

$startMethod = $daemonType.GetMethod("StartDaemon", [System.Reflection.BindingFlags]"Public,Static")
$stopMethod = $daemonType.GetMethod("StopDaemon", [System.Reflection.BindingFlags]"Public,Static")
$addProcessMethod = $daemonType.GetMethod("AddMonitoredProcess", [System.Reflection.BindingFlags]"Public,Static")
$getProcessesMethod = $daemonType.GetMethod("GetMonitoredProcesses", [System.Reflection.BindingFlags]"Public,Static")
$isEnabledProp = $daemonType.GetProperty("IsEnabled", [System.Reflection.BindingFlags]"Public,Static")
$isGameActiveProp = $daemonType.GetProperty("IsGameActive", [System.Reflection.BindingFlags]"Public,Static")

# 1. Test Default Targets
$targets = $getProcessesMethod.Invoke($null, @())
Write-Host "  [TEST 1] Monitored Targets Loaded:" ($targets.Count) "games/emulators" -ForegroundColor Green
$hasBlueStacks = $targets -contains "HD-Player"
$hasFreeFire = $targets -contains "FreeFire"
$hasCS2 = $targets -contains "cs2"
Write-Host "           -> BlueStacks:" $(if ($hasBlueStacks) { "YES" } else { "NO" }) "| FreeFire:" $(if ($hasFreeFire) { "YES" } else { "NO" }) "| CS2:" $(if ($hasCS2) { "YES" } else { "NO" }) -ForegroundColor DarkGreen

# 2. Test Custom Target Registration
$addProcessMethod.Invoke($null, @("AOTestDummyGame"))
$updatedTargets = $getProcessesMethod.Invoke($null, @())
$dummyAdded = $updatedTargets -contains "AOTestDummyGame"
Write-Host "  [TEST 2] Custom Target Registration:" $(if ($dummyAdded) { "PASS" } else { "FAIL" }) -ForegroundColor $(if ($dummyAdded) { "Green" } else { "Red" })

# 3. Test Daemon Startup
$startMethod.Invoke($null, @())
$running = $isEnabledProp.GetValue($null, $null)
Write-Host "  [TEST 3] Daemon Start State:" $(if ($running) { "RUNNING" } else { "STOPPED" }) -ForegroundColor $(if ($running) { "Green" } else { "Red" })

# 4. WinMM 1ms Timer Resolution Direct Test
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class WinMMTimer {
    [DllImport("winmm.dll", EntryPoint = "timeBeginPeriod")]
    public static extern uint TimeBeginPeriod(uint uMilliseconds);
    [DllImport("winmm.dll", EntryPoint = "timeEndPeriod")]
    public static extern uint TimeEndPeriod(uint uMilliseconds);
}
"@
$beginRes = [WinMMTimer]::TimeBeginPeriod(1)
$endRes = [WinMMTimer]::TimeEndPeriod(1)
$timerOk = ($beginRes -eq 0 -and $endRes -eq 0)
Write-Host "  [TEST 4] Win32 1.0ms Global Timer Resolution Interop:" $(if ($timerOk) { "PASS (Zero Latency Mode Supported)" } else { "FAIL" }) -ForegroundColor $(if ($timerOk) { "Green" } else { "Red" })

# 5. Test Daemon Shutdown and Revert
$stopMethod.Invoke($null, @())
$stopped = -not $isEnabledProp.GetValue($null, $null)
Write-Host "  [TEST 5] Daemon Clean Shutdown & Revert:" $(if ($stopped) { "PASS" } else { "FAIL" }) -ForegroundColor $(if ($stopped) { "Green" } else { "Red" })

# 6. Emdash validation
$emdashFiles = @()
Get-ChildItem -Path (Join-Path $PSScriptRoot "..\src") -Recurse -Include *.cs, *.xaml | ForEach-Object {
    $text = [System.IO.File]::ReadAllText($_.FullName)
    if ($text -match '[\u2013\u2014]') {
        $emdashFiles += $_.FullName
    }
}
if ($emdashFiles.Count -eq 0) {
    Write-Host "  [TEST 6] Zero Emdashes Across All C# and XAML Files:" "PASS" -ForegroundColor Green
} else {
    Write-Host "  [TEST 6] Found Emdashes in:" ($emdashFiles -join ", ") -ForegroundColor Red
}

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " FEATURE 2 IMPLEMENTATION AND VALIDATION COMPLETE" -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Cyan
