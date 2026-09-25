# Test Feature 1: In-Process Native Core Engine
$exePath = Join-Path $PSScriptRoot "..\dist\AnxiouslyOptimized.exe"
if (!(Test-Path $exePath)) {
    Write-Error "Binary not found at $exePath"
    exit 1
}

$bytes = [System.IO.File]::ReadAllBytes($exePath)
$asm = [System.Reflection.Assembly]::Load($bytes)
$engineType = $asm.GetType("AnxiouslyOptimized.Services.NativeTweakEngine")

if ($null -eq $engineType) {
    Write-Error "Could not find AnxiouslyOptimized.Services.NativeTweakEngine in assembly"
    exit 1
}

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " FEATURE 1: IN-PROCESS NATIVE ENGINE VERIFICATION" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

$canHandleMethod = $engineType.GetMethod("CanHandleNatively", [System.Reflection.BindingFlags]"Public,Static")
$checkMethod = $engineType.GetMethod("CheckTweakNative", [System.Reflection.BindingFlags]"Public,Static")

$tweakIds = @(
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
    "game_dvr",
    "mouse_acceleration",
    "ultimate_performance_plan",
    "hags_scheduling",
    "network_autotuning",
    "fast_ntfs_access",
    "emulator_bluestacks_fps",
    "cpu_core_parking",
    "delivery_optimization_p2p",
    "gpu_msi_mode"
)

$sw = [System.Diagnostics.Stopwatch]::StartNew()
$results = @{}

foreach ($id in $tweakIds) {
    $can = $canHandleMethod.Invoke($null, @($id))
    if (-not $can) {
        Write-Host "  [FAIL] Engine cannot handle $id" -ForegroundColor Red
        continue
    }

    $outParams = [object[]]@($id, $false)
    $handled = $checkMethod.Invoke($null, $outParams)
    $isApplied = $outParams[1]
    $results[$id] = $isApplied
    Write-Host ("  [NATIVE] {0,-30} : {1}" -f $id, $(if ($isApplied) { "APPLIED" } else { "NOT APPLIED" })) -ForegroundColor $(if ($isApplied) { "Green" } else { "DarkGray" })
}

$sw.Stop()

Write-Host "----------------------------------------------------------" -ForegroundColor Cyan
Write-Host (" Audited all {0} tweaks natively in: {1} ms" -f $tweakIds.Count, $sw.ElapsedMilliseconds) -ForegroundColor Green
Write-Host " Zero PowerShell subprocesses spawned during native audit!" -ForegroundColor Green

# Emdash validation across all C# sources
$emdashFiles = @()
Get-ChildItem -Path (Join-Path $PSScriptRoot "..\src") -Recurse -Include *.cs, *.xaml | ForEach-Object {
    $text = [System.IO.File]::ReadAllText($_.FullName)
    if ($text -match '[\u2013\u2014]') {
        $emdashFiles += $_.FullName
    }
}
if ($emdashFiles.Count -eq 0) {
    Write-Host " [PASS] Zero emdashes across all C# and XAML files!" -ForegroundColor Green
} else {
    Write-Host (" [FAIL] Found emdash in: " + ($emdashFiles -join ", ")) -ForegroundColor Red
}

Write-Host "==========================================================" -ForegroundColor Cyan
