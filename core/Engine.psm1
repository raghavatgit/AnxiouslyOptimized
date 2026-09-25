# ==============================================================================
#  ANXIOUSLYOPTIMIZED - CORE ORCHESTRATION ENGINE
#  Declarative tweak dispatcher, state evaluation, and batch preset execution
# ==============================================================================

[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidUsingWriteHost', '')]
[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSUseSingularNouns', '')]
[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSUseBOMForUnicodeEncodedFile', '')]
param()

$script:ConfigDir = Join-Path $PSScriptRoot "..\config"

function Get-DetectedOS {
    try {
        $os = Get-CimInstance Win32_OperatingSystem -ErrorAction SilentlyContinue
        $build = [int]$os.BuildNumber
        if ($build -ge 22000) { return "Win11" }
        return "Win10"
    } catch {
        if ([Environment]::OSVersion.Version.Build -ge 22000) { return "Win11" }
        return "Win10"
    }
}

function Get-TweaksConfig {
    param([string]$TargetOS = "Auto")
    
    if ($TargetOS -eq "Auto") {
        $TargetOS = Get-DetectedOS
    }
    
    $osFile = if ($TargetOS -eq "Win11") { "tweaks_win11.json" } else { "tweaks_win10.json" }
    $path = Join-Path $script:ConfigDir $osFile
    
    if (Test-Path $path) {
        return (Get-Content $path -Raw | ConvertFrom-Json)
    }
    
    # Fallback to legacy tweaks.json
    $fallback = Join-Path $script:ConfigDir "tweaks.json"
    if (Test-Path $fallback) {
        return (Get-Content $fallback -Raw | ConvertFrom-Json)
    }
    return @()
}

function Get-PresetsConfig {
    $path = Join-Path $script:ConfigDir "presets.json"
    if (Test-Path $path) {
        return (Get-Content $path -Raw | ConvertFrom-Json)
    }
    return @{}
}

function Test-TweakApplied {
    param([object]$Tweak)
    
    if (!$Tweak.checkScript) { return $false }
    try {
        $sb = [scriptblock]::Create($Tweak.checkScript)
        $res = & $sb
        return [bool]$res
    }
    catch {
        return $false
    }
}

function Invoke-TweakAction {
    param(
        [object]$Tweak,
        [ValidateSet("Apply", "Revert")][string]$Action = "Apply",
        [scriptblock]$LogCallback
    )
    
    $scriptToRun = if ($Action -eq "Apply") { $Tweak.applyScript } else { $Tweak.revertScript }
    if (!$scriptToRun) { return $false }
    
    $actionVerb = if ($Action -eq "Apply") { "Applying" } else { "Reverting" }
    $msg = "$actionVerb tweak: $($Tweak.title)..."
    
    if ($LogCallback) { & $LogCallback $msg }
    else { Write-Host $msg -ForegroundColor White }
    
    try {
        $sb = [scriptblock]::Create($scriptToRun)
        & $sb
        $statusMsg = "  [OK] $($Tweak.title) ($Action Completed)"
        if ($LogCallback) { & $LogCallback $statusMsg }
        else { Write-Host $statusMsg -ForegroundColor Green }
        return $true
    }
    catch {
        $errMsg = "  [ERROR] $actionVerb $($Tweak.title): $($_.Exception.Message)"
        if ($LogCallback) { & $LogCallback $errMsg }
        else { Write-Host $errMsg -ForegroundColor Red }
        return $false
    }
}

function Invoke-PresetBatch {
    param(
        [string]$PresetKey,
        [scriptblock]$LogCallback
    )
    
    $presets = Get-PresetsConfig
    $presetObj = $presets.$PresetKey
    
    if (!$presetObj) {
        $err = "Unknown preset: $PresetKey"
        if ($LogCallback) { & $LogCallback $err }
        return 0
    }
    
    $allTweaks = Get-TweaksConfig
    $appliedCount = 0
    
    $headerMsg = "Starting Profile: $($presetObj.name)"
    if ($LogCallback) { & $LogCallback "══════════════════════════════════════════" }
    if ($LogCallback) { & $LogCallback $headerMsg }
    if ($LogCallback) { & $LogCallback "══════════════════════════════════════════" }
    
    foreach ($tweakId in $presetObj.tweaks) {
        $match = $allTweaks | Where-Object { $_.id -eq $tweakId }
        if ($match) {
            $isApplied = Test-TweakApplied -Tweak $match
            if (!$isApplied) {
                $ok = Invoke-TweakAction -Tweak $match -Action "Apply" -LogCallback $LogCallback
                if ($ok) { $appliedCount++ }
            } else {
                if ($LogCallback) { & $LogCallback "  - Skipped (Already applied): $($match.title)" }
            }
        }
    }
    
    if ($LogCallback) { & $LogCallback "Finished applying profile. $appliedCount settings updated." }
    return $appliedCount
}

function Invoke-RevertAllTweaks {
    param([scriptblock]$LogCallback)
    
    $allTweaks = Get-TweaksConfig
    $revertedCount = 0
    
    if ($LogCallback) { & $LogCallback "══════════════════════════════════════════" }
    if ($LogCallback) { & $LogCallback "Resetting all settings back to original Windows defaults..." }
    if ($LogCallback) { & $LogCallback "══════════════════════════════════════════" }
    
    foreach ($tweak in $allTweaks) {
        $isApplied = Test-TweakApplied -Tweak $tweak
        if ($isApplied) {
            $ok = Invoke-TweakAction -Tweak $tweak -Action "Revert" -LogCallback $LogCallback
            if ($ok) { $revertedCount++ }
        }
    }
    
    if ($LogCallback) { & $LogCallback "Reset complete. $revertedCount settings restored to normal defaults." }
    return $revertedCount
}

Export-ModuleMember -Function Get-DetectedOS, Get-TweaksConfig, Get-PresetsConfig, Test-TweakApplied, Invoke-TweakAction, Invoke-PresetBatch, Invoke-RevertAllTweaks
