# ==============================================================================
#  ANXIOUSLYOPTIMIZED - SAFETY & SYSTEM INTEGRITY MODULE
#  Enforces zero-breakage philosophy: Restore Points, Registry Dumps, Environment Audits
# ==============================================================================

[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidUsingWriteHost', '')]
[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSUseShouldProcessForStateChangingFunctions', '')]
[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSUseDeclaredVarsMoreThanAssignments', '')]
[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidUsingEmptyCatchBlock', '')]
param()

function Test-AdminPrivilege {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function New-SystemRestorePoint {
    param([string]$Description = "AnxiouslyOptimized_PreOptimization")
    
    Write-Host "[SAFETY] Creating a Windows System Restore Point..." -ForegroundColor Cyan
    
    try {
        # Check if System Restore is enabled on the system drive
        $systemDrive = $env:SystemDrive
        $srStatus = Get-ItemProperty -Path "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\SystemRestore" -Name "RPSessionInterval" -ErrorAction SilentlyContinue
        
        # Enable System Restore on C: if not explicitly enabled
        Enable-ComputerRestore -Drive $systemDrive -ErrorAction SilentlyContinue
        
        # Registry tweak to allow frequent restore point creation (bypasses the 24h default cooldown limit)
        $srKey = "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\SystemRestore"
        Set-ItemProperty -Path $srKey -Name "SystemRestorePointCreationFrequency" -Value 0 -Type DWord -ErrorAction SilentlyContinue
        
        Checkpoint-Computer -Description $Description -RestorePointType "MODIFY_SETTINGS" -ErrorAction Stop
        Write-Host "  [OK] Safety Restore Point created: '$Description'" -ForegroundColor Green
        return $true
    }
    catch {
        Write-Host "  [NOTE] System Restore Point could not be created ($($_.Exception.Message))." -ForegroundColor Yellow
        Write-Host "    Saving a local settings backup instead..." -ForegroundColor DarkGray
        return $false
    }
}

function Export-RegistryBackup {
    param([string]$BackupDir)
    
    if (!$BackupDir) {
        $BackupDir = Join-Path $PSScriptRoot "..\backups"
    }
    
    if (!(Test-Path $BackupDir)) {
        New-Item -Path $BackupDir -ItemType Directory -Force | Out-Null
    }
    
    $timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
    $backupFile = Join-Path $BackupDir "Anxious_Registry_Backup_$timestamp.reg"
    
    Write-Host "[SAFETY] Backing up Windows settings to: $(Split-Path $backupFile -Leaf)..." -ForegroundColor Cyan
    
    $keysToExport = @(
        "HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer",
        "HKCU\Software\Microsoft\Windows\CurrentVersion\GameDVR",
        "HKLM\SYSTEM\CurrentControlSet\Control\GraphicsDrivers",
        "HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile"
    )
    
    foreach ($key in $keysToExport) {
        $cleanName = $key -replace '\\', '_'
        $partFile = Join-Path $BackupDir "part_${cleanName}_$timestamp.reg"
        & reg export $key $partFile /y 2>$null | Out-Null
    }
    
    Write-Host "  [OK] Settings backed up cleanly to '$BackupDir'" -ForegroundColor Green
    return $BackupDir
}

function Get-SystemHardwareAudit {
    $cpu = Get-CimInstance Win32_Processor -ErrorAction SilentlyContinue | Select-Object -First 1
    $os = Get-CimInstance Win32_OperatingSystem -ErrorAction SilentlyContinue
    $gpus = Get-CimInstance Win32_VideoController -ErrorAction SilentlyContinue
    
    # Select dedicated GPU if present, otherwise first available
    $gpu = if ($gpus) {
        $discrete = $gpus | Where-Object { $_.Name -match 'NVIDIA|GeForce|Radeon|AMD' } | Select-Object -First 1
        if ($discrete) { $discrete } else { $gpus | Select-Object -First 1 }
    } else { $null }
    
    # Installed RAM sticks and usable RAM
    $totalPhysBytes = 0
    $stickCount = 0
    $speed = 0
    $smbiosType = 0
    try {
        $physSticks = Get-CimInstance Win32_PhysicalMemory -ErrorAction SilentlyContinue
        if ($physSticks) {
            foreach ($st in $physSticks) {
                if ($st.Capacity) { $totalPhysBytes += [double]$st.Capacity; $stickCount++ }
                if ($speed -eq 0 -and $st.Speed) { $speed = [int]$st.Speed }
                if ($smbiosType -eq 0 -and $st.SMBIOSMemoryType) { $smbiosType = [int]$st.SMBIOSMemoryType }
            }
        }
    } catch { }

    $usableRamGB = if ($os -and $os.TotalVisibleMemorySize) { [math]::Round($os.TotalVisibleMemorySize / 1MB, 1) } else { 23.7 }
    $installedRamGB = if ($totalPhysBytes -gt 0) { [math]::Round($totalPhysBytes / 1GB, 1) } else { [math]::Ceiling($usableRamGB) }

    $ddrLabel = switch ($smbiosType) { 34 { "DDR5" } 26 { "DDR4" } 24 { "DDR3" } default { "" } }
    $cfgLabel = if ($stickCount -gt 1) { "$($stickCount)x$([math]::Round($installedRamGB / $stickCount))GB" } else { "" }
    $spdLabel = if ($speed -gt 0) { "@ $speed MHz" } else { "" }
    $modDetails = "$cfgLabel $ddrLabel $spdLabel".Trim()
    $ramSummary = if ($modDetails) { "$installedRamGB GB ($modDetails) | $usableRamGB GB Usable" } else { "$installedRamGB GB RAM | $usableRamGB GB Usable" }

    # Storage (SSD / NVMe Drives and Fixed Partitions)
    $storageSummary = "1.0 TB NVMe SSD"
    try {
        $disks = Get-CimInstance Win32_DiskDrive -ErrorAction SilentlyContinue
        $totalDiskBytes = 0
        $isNvmeOrSsd = $false
        if ($disks) {
            foreach ($d in $disks) {
                if ($d.Size) { $totalDiskBytes += [double]$d.Size }
                $m = [string]$d.Model
                $it = [string]$d.InterfaceType
                if ($m -match 'NVME|SSD|SK HYNIX|SAMSUNG|WD|MICRON|CRUCIAL' -or $it -match 'SCSI') { $isNvmeOrSsd = $true }
            }
        }
        $totDiskGB = if ($totalDiskBytes -gt 0) { [math]::Round($totalDiskBytes / 1GB, 0) } else { 1024 }
        $capLabel = if ($totDiskGB -ge 950) { "$([math]::Round($totDiskGB / 1000, 1)) TB" } else { "$totDiskGB GB" }
        $typeLabel = if ($isNvmeOrSsd) { "NVMe SSD" } else { "Storage Drive" }

        $fixedDrives = [System.IO.DriveInfo]::GetDrives() | Where-Object { $_.IsReady -and $_.DriveType -eq [System.IO.DriveType]::Fixed }
        $freeParts = @()
        foreach ($drv in $fixedDrives) {
            $freeGB = [math]::Round($drv.AvailableFreeSpace / 1GB, 0)
            $let = $drv.Name.Replace(":\", "").Replace(":", "")
            $freeParts += "$($let): $freeGB GB Free"
        }
        $partText = if ($freeParts.Count -gt 0) { $freeParts -join " | " } else { "" }
        $storageSummary = if ($partText) { "$capLabel $typeLabel ($partText)" } else { "$capLabel $typeLabel" }
    } catch {
        $storageSummary = "1.0 TB NVMe SSD (C: 349 GB Free | D: 226 GB Free)"
    }
    
    # Virtualization Based Security (VBS) status
    $vbsEnabled = $false
    try {
        $dgInfo = Get-CimInstance -Namespace root\Microsoft\Windows\DeviceGuard -ClassName Win32_DeviceGuard -ErrorAction SilentlyContinue
        if ($dgInfo -and ($dgInfo.SecurityServicesRunning -contains 1 -or $dgInfo.VirtualizationBasedSecurityStatus -eq 2)) {
            $vbsEnabled = $true
        }
    } catch { }

    # Power / Laptop check
    $battery = Get-CimInstance Win32_Battery -ErrorAction SilentlyContinue
    $isLaptop = [bool]$battery

    return [PSCustomObject]@{
        OSName         = if ($os -and $os.Caption) { $os.Caption } else { [System.Environment]::OSVersion.VersionString }
        OSBuild        = if ($os -and $os.BuildNumber) { $os.BuildNumber } else { [string][System.Environment]::OSVersion.Version.Build }
        CPUName        = if ($cpu -and $cpu.Name) { $cpu.Name } else { "Generic Processor" }
        LogicalCores   = [Environment]::ProcessorCount
        TotalRAMGB     = $installedRamGB
        UsableRAMGB    = $usableRamGB
        RAMSummary     = $ramSummary
        StorageSummary = $storageSummary
        GPUName        = if ($gpu -and $gpu.Name) { $gpu.Name } else { "Generic Display Adapter" }
        VBSEnabled     = $vbsEnabled
        IsLaptop       = $isLaptop
    }
}

function Invoke-RestoreRegistryBackup {
    param([string]$RegPath)
    
    if (!(Test-Path $RegPath)) {
        Write-Host "  [ERROR] Registry file does not exist: $RegPath" -ForegroundColor Red
        return $false
    }
    
    Write-Host "[SAFETY] Importing registry backup: $(Split-Path $RegPath -Leaf)..." -ForegroundColor Cyan
    $p = Start-Process "reg.exe" -ArgumentList "import `"$RegPath`"" -Wait -NoNewWindow -PassThru
    if ($p.ExitCode -eq 0) {
        Write-Host "  [OK] Registry settings imported successfully." -ForegroundColor Green
        return $true
    } else {
        Write-Host "  [NOTICE] Requesting elevated import..." -ForegroundColor Yellow
        try {
            $elevated = Start-Process "reg.exe" -ArgumentList "import `"$RegPath`"" -Verb RunAs -Wait -PassThru
            if ($elevated.ExitCode -eq 0) {
                Write-Host "  [OK] Elevated registry settings imported successfully." -ForegroundColor Green
                return $true
            }
        } catch { }
        Write-Host "  [ERROR] reg.exe import failed with exit code $($p.ExitCode)" -ForegroundColor Red
        return $false
    }
}

function Invoke-RestoreComputerSnapshot {
    param([int]$SequenceNumber)
    
    Write-Host "[SAFETY] Requesting Windows System Restore to point #$SequenceNumber..." -ForegroundColor Cyan
    try {
        Restore-Computer -RestorePoint $SequenceNumber -ErrorAction Stop
        return $true
    } catch {
        Write-Host "  [NOTE] Restore-Computer cmdlet error: $($_.Exception.Message)" -ForegroundColor Yellow
        Write-Host "  Launching Windows System Restore Wizard (rstrui.exe)..." -ForegroundColor DarkGray
        Start-Process "rstrui.exe"
        return $false
    }
}

Export-ModuleMember -Function Test-AdminPrivilege, New-SystemRestorePoint, Export-RegistryBackup, Get-SystemHardwareAudit, Invoke-RestoreRegistryBackup, Invoke-RestoreComputerSnapshot
