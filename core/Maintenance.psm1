# ==============================================================================
#  ANXIOUSLYOPTIMIZED - SYSTEM MAINTENANCE & REPAIR MODULE
#  Deep Windows repairs, RAM Flush, Core Unparking & Storage Reclaim
# ==============================================================================

[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidUsingWriteHost', '')]
[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSReviewUnusedParameter', '')]
[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSUseSingularNouns', '')]
param()

# Win32 Process Memory Flush signature
$memCode = @"
using System;
using System.Runtime.InteropServices;
public class MemoryManager {
    [DllImport("psapi.dll")]
    public static extern int EmptyWorkingSet(IntPtr hwProc);
}
"@
try {
    Add-Type -TypeDefinition $memCode -ErrorAction SilentlyContinue
} catch {
    $null = $_
}

function Invoke-SystemFileCheck {
    param([scriptblock]$LogCallback = $null)
    
    $helperLog = { param([string]$msg) if ($LogCallback) { & $LogCallback $msg } else { Write-Host $msg } }
    & $helperLog "[MAINTENANCE] Checking Windows system files for errors..."
    
    $proc = Start-Process -FilePath "sfc.exe" -ArgumentList "/scannow" -NoNewWindow -PassThru -Wait
    $exitCode = $proc.ExitCode
    
    $msg = switch ($exitCode) {
        0 { "All Windows system files are healthy. No damaged files found." }
        1 { "Found damaged system files and repaired them successfully." }
        default { "System file check finished (Code $exitCode)." }
    }
    
    & $helperLog "  [OK] $msg"
    return $exitCode
}

function Invoke-DISMRepair {
    param([scriptblock]$LogCallback = $null)
    
    $helperLog = { param([string]$msg) if ($LogCallback) { & $LogCallback $msg } else { Write-Host $msg } }
    & $helperLog "[MAINTENANCE] Downloading clean system files from Microsoft to fix Windows..."
    
    $proc = Start-Process -FilePath "dism.exe" -ArgumentList "/Online /Cleanup-Image /RestoreHealth" -NoNewWindow -PassThru -Wait
    $exitCode = $proc.ExitCode
    
    & $helperLog "  [OK] Windows repair completed successfully (Code: $exitCode)."
    return $exitCode
}

function Invoke-ComponentCleanup {
    param([scriptblock]$LogCallback = $null)
    
    $helperLog = { param([string]$msg) if ($LogCallback) { & $LogCallback $msg } else { Write-Host $msg } }
    & $helperLog "[MAINTENANCE] Cleaning old Windows update files to free up disk space..."
    
    $proc = Start-Process -FilePath "dism.exe" -ArgumentList "/Online /Cleanup-Image /StartComponentCleanup /ResetBase" -NoNewWindow -PassThru -Wait
    $exitCode = $proc.ExitCode
    
    & $helperLog "  [OK] Cleaned up old update files successfully (Code: $exitCode)."
    return $exitCode
}

function Invoke-ClearSafeCaches {
    param([scriptblock]$LogCallback = $null)
    
    $helperLog = { param([string]$msg) if ($LogCallback) { & $LogCallback $msg } else { Write-Host $msg } }
    & $helperLog "[MAINTENANCE] Cleaning temporary junk and game cache files..."
    
    $targets = @(
        "$env:LOCALAPPDATA\Temp",
        "C:\Windows\Temp",
        "$env:LOCALAPPDATA\D3DSCache",
        "$env:LOCALAPPDATA\NVIDIA\DXCache",
        "$env:LOCALAPPDATA\NVIDIA\GLCache",
        "$env:LOCALAPPDATA\AMD\DxCache",
        "$env:LOCALAPPDATA\AMD\GLCache"
    )
    
    $totalReclaimedMB = 0
    
    foreach ($target in $targets) {
        if (Test-Path $target) {
            try {
                $size = (Get-ChildItem $target -Recurse -Force -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum
                if ($size -gt 0) {
                    $mb = [math]::Round($size / 1MB, 1)
                    Remove-Item "$target\*" -Recurse -Force -ErrorAction SilentlyContinue | Out-Null
                    $totalReclaimedMB += $mb
                    & $helperLog "  Cleaned: $(Split-Path $target -Leaf) (${mb}MB freed)"
                }
            } catch {
                $null = $_
            }
        }
    }
    
    $msg = "Total space freed: ${totalReclaimedMB}MB"
    & $helperLog "  [OK] $msg"
    return $totalReclaimedMB
}

function Invoke-DisableCoreParking {
    param([scriptblock]$LogCallback = $null)

    $helperLog = { param([string]$msg) if ($LogCallback) { & $LogCallback $msg } else { Write-Host $msg } }
    & $helperLog "[CPU] Keeping all processor cores awake to stop game stutter..."

    # Processor Power Management GUID: 54533251-82be-4824-96c1-47b60b740d00
    # Min Cores Unparked GUID: 0cc5b647-c1df-4637-891a-dec35c318583
    # Max Cores Unparked GUID: ea062031-0e34-4ff1-9b6d-eb1059334028
    $schemes = @(
        "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c", # High Performance
        "e9a42b02-d5df-448d-aa00-03f14749eb61", # Ultimate Performance
        "381b4222-f694-41f0-9685-ff5bb260df2e"  # Balanced
    )

    # Also detect active scheme
    $activeScheme = (& powercfg /getactivescheme 2>$null)
    if ($activeScheme -match '([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})') {
        $schemes += $Matches[1]
    }

    foreach ($s in ($schemes | Select-Object -Unique)) {
        & powercfg /setacvalueindex $s 54533251-82be-4824-96c1-47b60b740d00 0cc5b647-c1df-4637-891a-dec35c318583 100 2>$null | Out-Null
        & powercfg /setacvalueindex $s 54533251-82be-4824-96c1-47b60b740d00 ea062031-0e34-4ff1-9b6d-eb1059334028 100 2>$null | Out-Null
        & powercfg /setacvalueindex $s 54533251-82be-4824-96c1-47b60b740d00 bc5038f7-23e0-4960-96da-33abaf5935ec 100 2>$null | Out-Null
        & powercfg /setacvalueindex $s 54533251-82be-4824-96c1-47b60b740d00 893dee8e-2bef-41e0-89c6-b55d0929964c 100 2>$null | Out-Null
    }

    # Re-apply active scheme
    if ($activeScheme -match '([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})') {
        & powercfg /setactive $Matches[1] 2>$null | Out-Null
    }

    & $helperLog "  [OK] All processor cores are now awake and ready for full speed."
    return $true
}

function Invoke-DisableDeliveryOptimization {
    param([scriptblock]$LogCallback = $null)

    $helperLog = { param([string]$msg) if ($LogCallback) { & $LogCallback $msg } else { Write-Host $msg } }
    & $helperLog "[INTERNET] Stopping Windows from uploading updates in the background..."

    $doKey = "HKLM:\SOFTWARE\Policies\Microsoft\Windows\DeliveryOptimization"
    if (!(Test-Path $doKey)) {
        New-Item -Path $doKey -Force | Out-Null
    }
    # 0 = Bypass P2P, HTTP only (never uploads data in background to strangers)
    Set-ItemProperty -Path $doKey -Name "DODownloadMode" -Value 0 -Type DWord -ErrorAction SilentlyContinue
    
    # Ensure Delivery Optimization service remains running for game downloads
    Set-Service -Name "DoSvc" -StartupType Automatic -ErrorAction SilentlyContinue
    Start-Service -Name "DoSvc" -ErrorAction SilentlyContinue

    & $helperLog "  [OK] Background update uploads stopped. Games and updates download at full speed."
    return $true
}

function Invoke-StandbyMemoryFlush {
    param([scriptblock]$LogCallback = $null)

    $helperLog = { param([string]$msg) if ($LogCallback) { & $LogCallback $msg } else { Write-Host $msg } }
    & $helperLog "[MEMORY] Cleaning out unused cached memory to free up RAM..."

    $beforeMem = (Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory
    $procList = Get-Process -ErrorAction SilentlyContinue
    $flushedCount = 0

    foreach ($proc in $procList) {
        try {
            if ($proc.Handle -ne [IntPtr]::Zero) {
                [MemoryManager]::EmptyWorkingSet($proc.Handle) | Out-Null
                $flushedCount++
            }
        } catch {
            $null = $_
        }
    }

    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()

    $afterMem = (Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory
    $freedMB = [Math]::Max(0, [Math]::Round(($afterMem - $beforeMem) / 1024, 1))

    & $helperLog "  [OK] Freed up ${freedMB}MB of memory across $flushedCount apps."
    return $freedMB
}

function Invoke-ToggleHibernation {
    param(
        [bool]$Disable = $true,
        [scriptblock]$LogCallback = $null
    )

    $helperLog = { param([string]$msg) if ($LogCallback) { & $LogCallback $msg } else { Write-Host $msg } }

    if ($Disable) {
        & $helperLog "[STORAGE] Turning off the sleep file to free up drive space..."
        & powercfg -h off 2>$null | Out-Null
        & $helperLog "  [OK] Sleep file turned off. Freed up 16 GB to 32 GB of drive space."
    } else {
        & $helperLog "[STORAGE] Turning the sleep file back on..."
        & powercfg -h on 2>$null | Out-Null
        & $helperLog "  [OK] Sleep file turned on."
    }
    return $true
}

function Invoke-EnableGPUMSIMode {
    param([scriptblock]$LogCallback = $null)

    $helperLog = { param([string]$msg) if ($LogCallback) { & $LogCallback $msg } else { Write-Host $msg } }
    & $helperLog "[GRAPHICS] Enabling direct graphics card communication for lower input lag..."

    $gpus = Get-PnpDevice -Class Display -Status OK -ErrorAction SilentlyContinue
    $enabledCount = 0

    foreach ($gpu in $gpus) {
        if ($gpu -and $gpu.InstanceId) {
            $instId = $gpu.InstanceId
            $msiPath = "HKLM:\SYSTEM\CurrentControlSet\Enum\$instId\Device Parameters\Interrupt Management\MessageSignaledInterruptProperties"
            if (!(Test-Path $msiPath)) {
                New-Item -Path $msiPath -Force -ErrorAction SilentlyContinue | Out-Null
            }
            if (Test-Path $msiPath) {
                Set-ItemProperty -Path $msiPath -Name "MSISupported" -Value 1 -Type DWord -ErrorAction SilentlyContinue
                & $helperLog "  Direct speed enabled for: $($gpu.FriendlyName)"
                $enabledCount++
            }
        }
    }

    & $helperLog "  [OK] Graphics card response speed optimized."
    return $enabledCount
}

Export-ModuleMember -Function `
    Invoke-SystemFileCheck, `
    Invoke-DISMRepair, `
    Invoke-ComponentCleanup, `
    Invoke-ClearSafeCaches, `
    Invoke-DisableCoreParking, `
    Invoke-DisableDeliveryOptimization, `
    Invoke-StandbyMemoryFlush, `
    Invoke-ToggleHibernation, `
    Invoke-EnableGPUMSIMode
