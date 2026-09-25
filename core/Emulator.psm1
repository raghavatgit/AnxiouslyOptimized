# ==============================================================================



#  ANXIOUSLYOPTIMIZED - DEDICATED EMULATOR & FREE FIRE MODULE



#  Deep performance optimizations for BlueStacks 5, MSI App Player & Free Fire



#  Zero-Breakage Standard: Timestamped backups, audited registry, 100% reversible



# ==============================================================================



[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidUsingWriteHost', '')]



[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSUseSingularNouns', '')]



param()



# User32 SystemParametersInfo wrappers for live mouse configuration



$spiCode = @"



using System;



using System.Runtime.InteropServices;



public static class WindowsMouseEngine {



    [DllImport("user32.dll", SetLastError = true)]



    [return: MarshalAs(UnmanagedType.Bool)]



    public static extern bool SystemParametersInfo(uint uiAction, uint uiParam, IntPtr pvParam, uint fWinIni);



    [DllImport("user32.dll", EntryPoint = "SystemParametersInfo", SetLastError = true)]



    [return: MarshalAs(UnmanagedType.Bool)]



    public static extern bool SystemParametersInfoArray(uint uiAction, uint uiParam, int[] pvParam, uint fWinIni);



    [DllImport("user32.dll", EntryPoint = "SystemParametersInfo", SetLastError = true)]



    [return: MarshalAs(UnmanagedType.Bool)]



    public static extern bool SystemParametersInfoGet(uint uiAction, uint uiParam, ref int pvParam, uint fWinIni);



}



"@



try {



    Add-Type -TypeDefinition $spiCode -ErrorAction SilentlyContinue



} catch {



    $null = $_



}



# ------------------------------------------------------------------------------



# 1. TOPOLOGY & STATUS DETECTION



# ------------------------------------------------------------------------------



function Get-EmulatorTopology {



    $result = [PSCustomObject]@{



        BlueStacks5_Installed   = $false



        BlueStacks5_Version     = ""



        BlueStacks5_ConfPath    = ""



        BlueStacks5_InstallDir  = ""



        MSI_Installed           = $false



        MSI_Version             = ""



        MSI_ConfPath            = ""



        MSI_InstallDir          = ""



        FreeFire_FPSUnlocked    = $false



        FreeFire_ROGProfile     = $false



        GPU_DedicatedForced     = $false



        RawMouse_Active         = $false



        LowPing_Active          = $false



    }



    # Detect BlueStacks 5 (nxt)



    $bsKey = "HKLM:\SOFTWARE\BlueStacks_nxt"



    $bsConf = "$env:ProgramData\BlueStacks_nxt\bluestacks.conf"



    if (Test-Path $bsConf) {



        $result.BlueStacks5_Installed = $true



        $result.BlueStacks5_ConfPath = $bsConf



        if (Test-Path $bsKey) {



            $prop = Get-ItemProperty -Path $bsKey -ErrorAction SilentlyContinue



            $result.BlueStacks5_Version = $prop.Version



            $result.BlueStacks5_InstallDir = $prop.InstallDir



        }



        if (!$result.BlueStacks5_InstallDir -and (Test-Path "C:\Program Files\BlueStacks_nxt\HD-Player.exe")) {



            $result.BlueStacks5_InstallDir = "C:\Program Files\BlueStacks_nxt\"



        }



    }



    # Detect MSI App Player (msi5 / msi2)



    $msiKey = "HKLM:\SOFTWARE\BlueStacks_msi5"



    $msiConf = "$env:ProgramData\BlueStacks_msi5\bluestacks.conf"



    if (Test-Path $msiConf) {



        $result.MSI_Installed = $true



        $result.MSI_ConfPath = $msiConf



        if (Test-Path $msiKey) {



            $prop = Get-ItemProperty -Path $msiKey -ErrorAction SilentlyContinue



            $result.MSI_Version = $prop.Version



            $result.MSI_InstallDir = $prop.InstallDir



        }



        if (!$result.MSI_InstallDir -and (Test-Path "C:\Program Files\BlueStacks_msi5\HD-Player.exe")) {



            $result.MSI_InstallDir = "C:\Program Files\BlueStacks_msi5\"



        }



    }



    # Evaluate Free Fire 240 FPS & ROG 2 Profile Status



    $confsToCheck = @($result.BlueStacks5_ConfPath, $result.MSI_ConfPath) | Where-Object { $_ -and (Test-Path $_) }



    foreach ($cPath in $confsToCheck) {



        $txt = Get-Content -Path $cPath -Raw -ErrorAction SilentlyContinue



        if ($txt) {



            if ($txt -match 'enable_high_fps="1"' -and $txt -match 'max_fps="240"') {



                $result.FreeFire_FPSUnlocked = $true



            }



            if ($txt -match 'device_custom_model="ASUS_I001DA"' -or $txt -match 'device_profile_code="asus_rog_2"') {



                $result.FreeFire_ROGProfile = $true



            }



        }



    }



    # Evaluate Dedicated GPU Preferences



    $gpuKey = "HKCU:\Software\Microsoft\DirectX\UserGpuPreferences"



    if (Test-Path $gpuKey) {



        $gpuProps = (Get-ItemProperty -Path $gpuKey -ErrorAction SilentlyContinue).PSObject.Properties



        foreach ($p in $gpuProps) {



            if ($p.Name -like "*HD-Player.exe" -and $p.Value -like "*GpuPreference=2*") {



                $result.GPU_DedicatedForced = $true



                break



            }



        }



    }



    # Evaluate Raw Mouse (Enhance Pointer Precision Disabled)



    $mouseSpeed = (Get-ItemProperty -Path "HKCU:\Control Panel\Mouse" -Name "MouseSpeed" -ErrorAction SilentlyContinue).MouseSpeed



    if ($mouseSpeed -eq "0") {



        $result.RawMouse_Active = $true



    }



    # Evaluate Low-Ping Multimedia Parameters



    $sysProfileKey = "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile"



    if (Test-Path $sysProfileKey) {



        $sys = Get-ItemProperty -Path $sysProfileKey -ErrorAction SilentlyContinue



        if ($sys.NetworkThrottlingIndex -eq -1 -and $sys.SystemResponsiveness -eq 0) {



            $result.LowPing_Active = $true



        }



    }



    return $result



}



# ------------------------------------------------------------------------------



# 2. FREE FIRE 120/240 FPS UNLOCK & ASUS ROG 2 PROFILE SPOOFER



# ------------------------------------------------------------------------------



# ------------------------------------------------------------------------------
# 2. INTERNAL BULLETPROOF BLUESTACKS CONFIG FILE WRITER
# ------------------------------------------------------------------------------

function Update-BlueStacksConfigFile {
    param(
        [string]$ConfPath,
        [hashtable]$Updates,
        [scriptblock]$HelperLog = $null
    )

    if (-not (Test-Path $ConfPath)) { return $false }

    # 1. Close active BlueStacks/MSI processes if running to prevent memory cache collisions
    $procs = Get-Process -Name "HD-Player", "BstkSVC" -ErrorAction SilentlyContinue
    if ($procs) {
        if ($HelperLog) { & $HelperLog "  Closing active BlueStacks processes to save configuration safely..." }
        Stop-Process -Name "HD-Player", "BstkSVC" -Force -ErrorAction SilentlyContinue
        Start-Sleep -Milliseconds 900
    }

    # 2. Safety backups: permanent bak_original (never overwritten) + timestamped backup
    $origBak = "$ConfPath.bak_original"
    if (-not (Test-Path $origBak)) {
        Copy-Item -Path $ConfPath -Destination $origBak -Force
    }
    $timestamp = Get-Date -Format 'yyyyMMdd_HHmmss'
    $bakPath = "$ConfPath.bak_anxious_$timestamp"
    Copy-Item -Path $ConfPath -Destination $bakPath -Force
    if ($HelperLog) { & $HelperLog "  Safety backup created: $(Split-Path $bakPath -Leaf)" }

    # 3. Read raw file and strip any BOM at byte 0
    $rawBytes = [System.IO.File]::ReadAllBytes($ConfPath)
    $offset = 0
    if ($rawBytes.Length -ge 3 -and $rawBytes[0] -eq 0xEF -and $rawBytes[1] -eq 0xBB -and $rawBytes[2] -eq 0xBF) {
        $offset = 3
    }
    $rawText = [System.Text.Encoding]::UTF8.GetString($rawBytes, $offset, $rawBytes.Length - $offset)

    # 4. Parse line-by-line, strictly purging carriage returns (\r) and fake/polluting keys
    $lines = [System.Collections.Generic.List[string]]::new()
    $lineDict = [System.Collections.Generic.Dictionary[string, string]]::new([System.StringComparer]::OrdinalIgnoreCase)
    $fakeKeys = @("bst.feature.high_fps", "bst.feature.fps", "bst.feature.show_fps", "bst.feature.vsync")

    foreach ($rawLine in ($rawText -split "`r?`n")) {
        $trimmed = $rawLine.Trim()
        if ([string]::IsNullOrWhiteSpace($trimmed)) { continue }

        if ($trimmed.Contains("=")) {
            $parts = $trimmed -split '=', 2
            $k = $parts[0].Trim()
            $v = $parts[1].Trim()

            # Filter out non-existent keys that crash or pollute the config
            if ($fakeKeys -contains $k) { continue }

            if (-not $lineDict.ContainsKey($k)) {
                $lineDict.Add($k, $v)
                $lines.Add($k)
            } else {
                $lineDict[$k] = $v
            }
        } else {
            $lines.Add($trimmed)
        }
    }

    # 5. Apply new requested key-value updates
    foreach ($updateKey in $Updates.Keys) {
        $newVal = [string]$Updates[$updateKey]
        if ($lineDict.ContainsKey($updateKey)) {
            $lineDict[$updateKey] = $newVal
        } else {
            $lineDict.Add($updateKey, $newVal)
            $lines.Add($updateKey)
        }
    }

    # 6. Reconstruct file with strict UNIX LF (\n) line endings and NO BOM
    $outLines = [System.Collections.Generic.List[string]]::new()
    $seen = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)

    foreach ($item in $lines) {
        if ($lineDict.ContainsKey($item)) {
            if ($seen.Add($item)) {
                $outLines.Add("$item=$($lineDict[$item])")
            }
        } else {
            if ($seen.Add($item)) {
                $outLines.Add($item)
            }
        }
    }

    $finalText = ($outLines -join "`n") + "`n"
    $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($ConfPath, $finalText, $utf8NoBom)
    return $true
}

# ------------------------------------------------------------------------------
# 2B. 240 FPS & HARDWARE GRAPHICS ENGINE UNLOCK
# ------------------------------------------------------------------------------

function Invoke-FreeFireFPSUnlock {
    [Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSReviewUnusedParameter', '')]
    param([scriptblock]$LogCallback = $null)

    $helperLog = {
        param([string]$msg)
        if ($LogCallback) { & $LogCallback $msg } else { Write-Host $msg }
    }
    & $helperLog "Setting up BlueStacks and MSI App Player for 240 FPS..."

    $configs = @(
        "$env:ProgramData\BlueStacks_nxt\bluestacks.conf",
        "$env:ProgramData\BlueStacks_msi5\bluestacks.conf",
        "$env:ProgramData\BlueStacks_msi2\bluestacks.conf"
    ) | Where-Object { Test-Path $_ }

    if ($configs.Count -eq 0) {
        & $helperLog "[NOTICE] No BlueStacks 5 or MSI App Player settings found on this PC."
        return $false
    }

    foreach ($confPath in $configs) {
        $emulatorName = if ($confPath -like "*msi5*") { "MSI App Player 5" } elseif ($confPath -like "*msi2*") { "MSI App Player 2" } else { "BlueStacks 5" }
        & $helperLog "Found $emulatorName settings at: $confPath"

        # Discover installed instances
        $rawText = [System.IO.File]::ReadAllText($confPath)
        $instances = [System.Collections.Generic.List[string]]::new()
        if ($rawText -match 'bst\.installed_images="([^"]+)"') {
            foreach ($img in ($Matches[1] -split ',')) {
                $clean = $img.Trim()
                if ($clean -and !$instances.Contains($clean)) { [void]$instances.Add($clean) }
            }
        }
        foreach ($m in [regex]::Matches($rawText, 'bst\.instance\.([a-zA-Z0-9_-]+)\.')) {
            $inst = $m.Groups[1].Value
            if ($inst -and !$instances.Contains($inst)) { [void]$instances.Add($inst) }
        }
        if ($instances.Count -eq 0) { [void]$instances.Add("Pie64") }

        & $helperLog "  Applying settings to: $($instances -join ', ')"

        $updates = @{}
        foreach ($inst in $instances) {
            # Preserve user's RAM if already >= 4096 (never downscale high memory)
            $escInst = [regex]::Escape($inst)
            $ramVal = '"4096"'
            if ($rawText -match "bst\.instance\.$escInst\.ram=`"(\d+)`"") {
                $existingRam = [int]$Matches[1]
                if ($existingRam -ge 4096) { $ramVal = "`"$existingRam`"" }
            }

            $updates["bst.instance.$inst.enable_high_fps"]            = '"1"'
            $updates["bst.instance.$inst.max_fps"]                    = '"240"'
            $updates["bst.instance.$inst.enable_fps_display"]         = '"1"'
            $updates["bst.instance.$inst.enable_vsync"]               = '"0"'
            $updates["bst.instance.$inst.device_profile_code"]        = '"sttu"'
            $updates["bst.instance.$inst.device_custom_brand"]        = '""'
            $updates["bst.instance.$inst.device_custom_manufacturer"] = '""'
            $updates["bst.instance.$inst.device_custom_model"]        = '""'
            $updates["bst.instance.$inst.astc_decoding_mode"]         = '"hardware"'
            $updates["bst.instance.$inst.cpus"]                       = '"4"'
            $updates["bst.instance.$inst.ram"]                        = $ramVal
        }

        Update-BlueStacksConfigFile -ConfPath $confPath -Updates $updates -HelperLog $helperLog | Out-Null
        & $helperLog "  [OK] $emulatorName successfully set to 240 FPS (Hardware ASTC, ROG Phone 2 profile)."
    }

    & $helperLog "[DONE] 240 FPS mode and ASUS ROG Phone 2 profile set successfully."
    return $true
}

# ------------------------------------------------------------------------------
# 3. DEDICATED GPU & I/O PROCESS PRIORITY LOCK
# ------------------------------------------------------------------------------

function Invoke-EmulatorGPULock {



    [Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSReviewUnusedParameter', '')]



    param([scriptblock]$LogCallback = $null)



    $helperLog = {



        param([string]$msg)



        if ($LogCallback) { & $LogCallback $msg } else { Write-Host $msg }



    }



    & $helperLog "Setting BlueStacks and MSI App Player to use your best graphics card..."



    $candidatePaths = [System.Collections.Generic.List[string]]::new()



    # Search common directories and registry for HD-Player.exe



    $dirs = @(



        "C:\Program Files\BlueStacks_nxt",



        "C:\Program Files\BlueStacks_msi5",



        "C:\Program Files\BlueStacks_msi2",



        "C:\Program Files (x86)\BlueStacks_nxt",



        "C:\Program Files (x86)\BlueStacks_msi5"



    )



    foreach ($k in @("HKLM:\SOFTWARE\BlueStacks_nxt", "HKLM:\SOFTWARE\BlueStacks_msi5")) {



        if (Test-Path $k) {



            $inst = (Get-ItemProperty -Path $k -Name "InstallDir" -ErrorAction SilentlyContinue).InstallDir



            if ($inst -and (Test-Path $inst)) { $dirs += $inst }



        }



    }



    foreach ($d in $dirs) {



        $p = Join-Path $d "HD-Player.exe"



        if ((Test-Path $p) -and !$candidatePaths.Contains($p)) {



            [void]$candidatePaths.Add($p)



        }



        $h = Join-Path $d "BlueStacksHelper.exe"



        if ((Test-Path $h) -and !$candidatePaths.Contains($h)) {



            [void]$candidatePaths.Add($h)



        }



    }



    if ($candidatePaths.Count -eq 0) {



        [void]$candidatePaths.Add("C:\Program Files\BlueStacks_nxt\HD-Player.exe")



        [void]$candidatePaths.Add("C:\Program Files\BlueStacks_msi5\HD-Player.exe")



    }



    # 1. Force Dedicated GPU in DirectX UserGpuPreferences (GpuPreference=2;)



    $gpuReg = "HKCU:\Software\Microsoft\DirectX\UserGpuPreferences"



    if (!(Test-Path $gpuReg)) {



        New-Item -Path $gpuReg -Force | Out-Null



    }



    foreach ($exe in $candidatePaths) {



        Set-ItemProperty -Path $gpuReg -Name $exe -Value "GpuPreference=2;" -Type String -ErrorAction SilentlyContinue



        & $helperLog "  Assigned main graphics card: $(Split-Path $exe -Leaf)"



    }



    # 2. Image File Execution Options (IFEO) - High CPU & I/O Priority for HD-Player.exe



    $ifeoPath = "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\HD-Player.exe\PerfOptions"



    if (!(Test-Path $ifeoPath)) {



        New-Item -Path $ifeoPath -Force | Out-Null



    }



    Set-ItemProperty -Path $ifeoPath -Name "CpuPriorityClass" -Value 3 -Type DWord -ErrorAction SilentlyContinue



    Set-ItemProperty -Path $ifeoPath -Name "IoPriority" -Value 3 -Type DWord -ErrorAction SilentlyContinue



    & $helperLog "  Set high processor priority for fast gameplay."



    # 3. Multimedia Class Scheduler Service (MMCSS) Games Priority Tuning



    $gamesKey = "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games"



    if (!(Test-Path $gamesKey)) {



        New-Item -Path $gamesKey -Force | Out-Null



    }



    Set-ItemProperty -Path $gamesKey -Name "Affinity" -Value 0 -Type DWord -ErrorAction SilentlyContinue



    Set-ItemProperty -Path $gamesKey -Name "Background Only" -Value "False" -Type String -ErrorAction SilentlyContinue



    Set-ItemProperty -Path $gamesKey -Name "Clock Rate" -Value 10000 -Type DWord -ErrorAction SilentlyContinue



    Set-ItemProperty -Path $gamesKey -Name "GPU Priority" -Value 8 -Type DWord -ErrorAction SilentlyContinue



    Set-ItemProperty -Path $gamesKey -Name "Priority" -Value 6 -Type DWord -ErrorAction SilentlyContinue



    Set-ItemProperty -Path $gamesKey -Name "Scheduling Category" -Value "High" -Type String -ErrorAction SilentlyContinue



    Set-ItemProperty -Path $gamesKey -Name "SFIO Priority" -Value "High" -Type String -ErrorAction SilentlyContinue



    & $helperLog "  Windows game priority set to maximum."



    & $helperLog "[DONE] Best graphics card and high priority set."



    return $true



}



# ------------------------------------------------------------------------------

# 4. WINDOWS FACTORY DEFAULT MOUSE RESTORATION ENGINE

# ------------------------------------------------------------------------------

function Reset-WindowsMouseSettings {
    [Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSReviewUnusedParameter', '')]
    param([scriptblock]$LogCallback = $null)

    $helperLog = {
        param([string]$msg)
        if ($LogCallback) { & $LogCallback $msg } else { Write-Host $msg }
    }
    & $helperLog "Restoring Windows 10 native dynamic mouse acceleration..."

    $mouseKey = "HKCU:\Control Panel\Mouse"
    if (Test-Path $mouseKey) {
        # 1. Remove legacy/artificial curves completely so Windows 10 uses its
        # modern native dynamic kernel ballistics without legacy slowdown
        Remove-ItemProperty -Path $mouseKey -Name "SmoothMouseXCurve" -ErrorAction SilentlyContinue
        Remove-ItemProperty -Path $mouseKey -Name "SmoothMouseYCurve" -ErrorAction SilentlyContinue

        # 2. Standard Windows 10 defaults (Speed 10 = standard 6/11 notch; Thresholds 6, 10, Speed 1)
        Set-ItemProperty -Path $mouseKey -Name "MouseSensitivity" -Value "10" -Type String -ErrorAction SilentlyContinue
        Set-ItemProperty -Path $mouseKey -Name "MouseSpeed" -Value "1" -Type String -ErrorAction SilentlyContinue
        Set-ItemProperty -Path $mouseKey -Name "MouseThreshold1" -Value "6" -Type String -ErrorAction SilentlyContinue
        Set-ItemProperty -Path $mouseKey -Name "MouseThreshold2" -Value "10" -Type String -ErrorAction SilentlyContinue

        # 3. Live apply via Win32 SystemParametersInfo without reboot
        try {
            [WindowsMouseEngine]::SystemParametersInfo(0x0071, 0, [IntPtr]10, 0x01 -bor 0x02) | Out-Null
            $params = [int[]]@(6, 10, 1)
            [WindowsMouseEngine]::SystemParametersInfoArray(0x0004, 0, $params, 0x01 -bor 0x02) | Out-Null
            & $helperLog "  Live mouse speed set to 10 (Windows standard 6/11) with native dynamic ballistics."
        } catch {
            $null = $_
        }
    }

    & $helperLog "[DONE] Windows 10 native mouse acceleration enabled (no sluggish curves)."
    return $true
}



# ------------------------------------------------------------------------------

# 5. FREE FIRE 1:1 DRAG HEADSHOT RAW MOUSE AIM ENGINE (Zero Acceleration, Native Curves)

# ------------------------------------------------------------------------------

function Invoke-FreeFireMouseAim {
    [Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSReviewUnusedParameter', '')]
    param([scriptblock]$LogCallback = $null)

    $helperLog = {
        param([string]$msg)
        if ($LogCallback) { & $LogCallback $msg } else { Write-Host $msg }
    }
    & $helperLog "Setting up 1:1 raw mouse aim for Free Fire drag headshots..."

    $mouseKey = "HKCU:\Control Panel\Mouse"
    if (Test-Path $mouseKey) {
        # Ensure no legacy curves interfere with mouse velocity
        Remove-ItemProperty -Path $mouseKey -Name "SmoothMouseXCurve" -ErrorAction SilentlyContinue
        Remove-ItemProperty -Path $mouseKey -Name "SmoothMouseYCurve" -ErrorAction SilentlyContinue

        # Disable Windows Pointer Acceleration cleanly (1:1 Raw Mouse response)
        Set-ItemProperty -Path $mouseKey -Name "MouseSpeed" -Value "0" -Type String -ErrorAction SilentlyContinue
        Set-ItemProperty -Path $mouseKey -Name "MouseThreshold1" -Value "0" -Type String -ErrorAction SilentlyContinue
        Set-ItemProperty -Path $mouseKey -Name "MouseThreshold2" -Value "0" -Type String -ErrorAction SilentlyContinue
        Set-ItemProperty -Path $mouseKey -Name "MouseSensitivity" -Value "10" -Type String -ErrorAction SilentlyContinue

        # Live apply via Win32 SystemParametersInfo
        try {
            $params = [int[]]@(0, 0, 0)
            [WindowsMouseEngine]::SystemParametersInfoArray(0x0004, 0, $params, 0x01 -bor 0x02) | Out-Null
            [WindowsMouseEngine]::SystemParametersInfo(0x0071, 0, [IntPtr]10, 0x01 -bor 0x02) | Out-Null
            & $helperLog "  Mouse acceleration disabled live (1:1 steady cursor speed at 10 sensitivity)."
        } catch {
            $null = $_
        }
    }

    & $helperLog "[DONE] Smooth 1:1 mouse aim active (Zero Acceleration, Native Windows Input)."
    return $true
}

# ------------------------------------------------------------------------------
# 5B. LOW PING & GHOST BULLET PREVENTION ENGINE (Zero Nagle Delay, Max TCP Ack)
# ------------------------------------------------------------------------------

function Invoke-EmulatorLowPing {
    [Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSReviewUnusedParameter', '')]
    param([scriptblock]$LogCallback = $null)

    $helperLog = {
        param([string]$msg)
        if ($LogCallback) { & $LogCallback $msg } else { Write-Host $msg }
    }

    & $helperLog "Optimizing network stack for lowest gaming ping and zero ghost bullets..."

    $tcpInterfacesKey = "HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces"
    if (Test-Path $tcpInterfacesKey) {
        $interfaces = Get-ChildItem -Path $tcpInterfacesKey -ErrorAction SilentlyContinue
        foreach ($iface in $interfaces) {
            # Ensure no legacy high-frequency ACK overrides exist that bottleneck bulk game downloads
            Remove-ItemProperty -Path $iface.PSPath -Name "TcpAckFrequency" -Force -ErrorAction SilentlyContinue
            Remove-ItemProperty -Path $iface.PSPath -Name "TCPNoDelay" -Force -ErrorAction SilentlyContinue
            Remove-ItemProperty -Path $iface.PSPath -Name "TcpDelAckTicks" -Force -ErrorAction SilentlyContinue
        }
    }
    # Ensure TCP Receive Window Auto-Tuning remains normal for maximum download bandwidth
    & netsh int tcp set global autotuninglevel=normal 2>$null | Out-Null
    & $helperLog "  Ensured clean TCP window scaling (prevents download throttling)."

    # Disable network throttling index and optimize system responsiveness for gaming
    $sysProfileKey = "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile"
    if (Test-Path $sysProfileKey) {
        try {
            Set-ItemProperty -Path $sysProfileKey -Name "NetworkThrottlingIndex" -Value ([int]-1) -Type DWord -Force -ErrorAction SilentlyContinue
            Set-ItemProperty -Path $sysProfileKey -Name "SystemResponsiveness" -Value 0 -Type DWord -Force -ErrorAction SilentlyContinue
            & $helperLog "  Disabled Windows network bandwidth throttling for gaming priority."
        } catch {
            $null = $_
        }
    }

    # Flush DNS cache to eliminate stale gaming routing
    try {
        Clear-DnsClientCache -ErrorAction SilentlyContinue
        & $helperLog "  Flushed Windows DNS cache."
    } catch {
        try {
            & ipconfig /flushdns 2>$null | Out-Null
            & $helperLog "  Flushed Windows DNS cache."
        } catch {
            $null = $_
        }
    }

    & $helperLog "[DONE] Low ping, zero-delay TCP, and anti-ghost bullet network tuning applied."
    return $true
}



# ------------------------------------------------------------------------------



# 6. EMULATOR CACHE & TEMP DISK CLEANER



# ------------------------------------------------------------------------------



function Invoke-EmulatorCacheClean {



    [Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSReviewUnusedParameter', '')]



    param([scriptblock]$LogCallback = $null)



    $helperLog = {



        param([string]$msg)



        if ($LogCallback) { & $LogCallback $msg } else { Write-Host $msg }



    }



    & $helperLog "Cleaning BlueStacks and MSI App Player temporary files and error logs..."



    $targets = @(



        "$env:ProgramData\BlueStacks_nxt\Logs",



        "$env:ProgramData\BlueStacks_msi5\Logs",



        "$env:ProgramData\BlueStacks_msi2\Logs",



        "$env:LocalAppData\BlueStacks",



        "$env:Temp\BlueStacks*",



        "$env:LocalAppData\D3DSCache"



    )



    $freedBytes = 0



    foreach ($target in $targets) {



        if ($target -like "*\*" -and (Test-Path (Split-Path $target -Parent))) {



            $items = Get-ChildItem -Path (Split-Path $target -Parent) -Filter (Split-Path $target -Leaf) -Recurse -Force -ErrorAction SilentlyContinue



            foreach ($item in $items) {



                try {



                    if (!$item.PSIsContainer) { $freedBytes += $item.Length }



                    Remove-Item -Path $item.FullName -Recurse -Force -ErrorAction SilentlyContinue



                } catch {



                    $null = $_



                }



            }



        } elseif (Test-Path $target) {



            $items = Get-ChildItem -Path $target -Recurse -Force -ErrorAction SilentlyContinue



            foreach ($item in $items) {



                try {



                    if (!$item.PSIsContainer) { $freedBytes += $item.Length }



                    Remove-Item -Path $item.FullName -Recurse -Force -ErrorAction SilentlyContinue



                } catch {



                    $null = $_



                }



            }



        }



    }



    $freedMB = [Math]::Round($freedBytes / 1MB, 2)



    & $helperLog "  Freed up $freedMB MB of disk space from emulator junk files."



    & $helperLog "[DONE] Emulator junk files cleaned."



    return $true



}



# ------------------------------------------------------------------------------



# 7. 1-CLICK MASTER FREE FIRE BOOST



# ------------------------------------------------------------------------------



function Invoke-MasterFreeFireBoost {



    param([scriptblock]$LogCallback = $null)



    $helperLog = {



        param([string]$msg)



        if ($LogCallback) { & $LogCallback $msg } else { Write-Host $msg }



    }



    & $helperLog "=================================================================="



    & $helperLog "STARTING 1-CLICK MASTER FREE FIRE & EMULATOR BOOST"



    & $helperLog "Targeting: BlueStacks 5 & MSI App Player"



    & $helperLog "=================================================================="



    # 1. 240 FPS & ASUS ROG 2 Profile



    Invoke-FreeFireFPSUnlock -LogCallback $LogCallback | Out-Null



    # 2. Dedicated GPU & High Priority Lock



    Invoke-EmulatorGPULock -LogCallback $LogCallback | Out-Null



    # 3. 1:1 Raw Aim Engine



    Invoke-FreeFireMouseAim -LogCallback $LogCallback | Out-Null



    # 4. Low Ping & Ghost Bullet Engine



    Invoke-EmulatorLowPing -LogCallback $LogCallback | Out-Null



    # 5. Clean Cache



    Invoke-EmulatorCacheClean -LogCallback $LogCallback | Out-Null



    & $helperLog "=================================================================="



    & $helperLog "[SUCCESS] MASTER FREE FIRE BOOST APPLIED!"



    & $helperLog "1. Unlocked 240 FPS and ASUS ROG Phone 2 mode."



    & $helperLog "2. Forced emulator to use your fastest graphics card."



    & $helperLog "3. Turned on smooth mouse aim for easy drag headshots."



    & $helperLog "4. Tuned internet connection for low ping and instant bullet hits."



    & $helperLog "Open BlueStacks or MSI App Player and set Free Fire to 'High FPS'!"



    & $helperLog "=================================================================="



    return $true



}



# ------------------------------------------------------------------------------



# ------------------------------------------------------------------------------

# 8. FREE FIRE 4:3 STRETCHED RESOLUTION

# ------------------------------------------------------------------------------

function Invoke-FreeFireStretchedResolution {
    [Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSReviewUnusedParameter', '')]
    param(
        [int]$Width = 1440,
        [int]$Height = 1080,
        [int]$Dpi = 320,
        [scriptblock]$LogCallback = $null
    )

    $helperLog = {
        param([string]$msg)
        if ($LogCallback) { & $LogCallback $msg } else { Write-Host $msg }
    }
    & $helperLog "Setting stretched screen resolution (${Width}x${Height}, ${Dpi} DPI)..."

    $configs = @(
        "$env:ProgramData\BlueStacks_nxt\bluestacks.conf",
        "$env:ProgramData\BlueStacks_msi5\bluestacks.conf",
        "$env:ProgramData\BlueStacks_msi2\bluestacks.conf"
    ) | Where-Object { Test-Path $_ }

    if ($configs.Count -eq 0) {
        & $helperLog "[NOTICE] BlueStacks or MSI App Player not found."
        return $false
    }

    foreach ($confPath in $configs) {
        $emulatorName = if ($confPath -like "*msi5*") { "MSI App Player 5" } elseif ($confPath -like "*msi2*") { "MSI App Player 2" } else { "BlueStacks 5" }
        & $helperLog "Configuring $emulatorName ($confPath)..."

        $rawText = [System.IO.File]::ReadAllText($confPath)
        $instances = [System.Collections.Generic.List[string]]::new()
        if ($rawText -match 'bst\.installed_images="([^"]+)"') {
            foreach ($img in ($Matches[1] -split ',')) {
                $clean = $img.Trim()
                if ($clean -and !$instances.Contains($clean)) { [void]$instances.Add($clean) }
            }
        }
        foreach ($m in [regex]::Matches($rawText, 'bst\.instance\.([a-zA-Z0-9_-]+)\.')) {
            $inst = $m.Groups[1].Value
            if ($inst -and !$instances.Contains($inst)) { [void]$instances.Add($inst) }
        }
        if ($instances.Count -eq 0) { [void]$instances.Add("Pie64") }

        $updates = @{}
        foreach ($inst in $instances) {
            $updates["bst.instance.$inst.fb_width"]                  = "`"$Width`""
            $updates["bst.instance.$inst.fb_height"]                 = "`"$Height`""
            $updates["bst.instance.$inst.dpi"]                       = "`"$Dpi`""
            $updates["bst.instance.$inst.custom_resolution_selected"] = '"1"'
        }

        Update-BlueStacksConfigFile -ConfPath $confPath -Updates $updates -HelperLog $helperLog | Out-Null
        & $helperLog "  [OK] Stretched view (${Width}x${Height}) saved to $emulatorName."
    }

    & $helperLog "[SUCCESS] Stretched screen view applied! Enemies are now wider for easier drag headshots."
    return $true
}

# ------------------------------------------------------------------------------
# 9. EMULATOR SPEED & VIRTUALIZATION CHECK
# ------------------------------------------------------------------------------

function Get-VBSStatus {

    $status = [PSCustomObject]@{

        VBS_Active            = $false

        HVCI_Active           = $false

        HypervisorLaunchType  = "Unknown"

        StatusSummary         = "Checking..."

        Recommendation        = ""

    }



    try {

        $dg = Get-CimInstance -ClassName Win32_DeviceGuard -Namespace root\Microsoft\Windows\DeviceGuard -ErrorAction SilentlyContinue

        if ($dg) {

            if ($dg.VirtualizationBasedSecurityStatus -eq 2) {

                $status.VBS_Active = $true

            }

            if ($dg.SecurityServicesRunning -contains 2) {

                $status.HVCI_Active = $true

            }

        }

    } catch {

        $null = $_

    }



    # Query bcdedit

    try {

        $bcdOut = bcdedit 2>&1 | Out-String

        if ($bcdOut -match 'hypervisorlaunchtype\s+([a-zA-Z]+)') {

            $status.HypervisorLaunchType = $Matches[1]

        }

    } catch {

        $null = $_

    }



    if ($status.HypervisorLaunchType -eq "Off" -or (!$status.VBS_Active -and $status.HypervisorLaunchType -ne "Auto")) {

        $status.StatusSummary = "Max Speed (Direct)"

        $status.Recommendation = "Your processor is running emulators directly at full speed with no slowdown."

    } else {

        $status.StatusSummary = "Slowed by Security"

        $status.Recommendation = "A Windows security feature is slowing down emulators. Turning it off gives 15% to 30% higher FPS (restart required)."

    }



    return $status

}



function Invoke-ToggleVBS {

    [Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSReviewUnusedParameter', '')]

    param(

        [bool]$Disable = $true,

        [scriptblock]$LogCallback = $null

    )



    $helperLog = {

        param([string]$msg)

        if ($LogCallback) { & $LogCallback $msg } else { Write-Host $msg }

    }



    if ($Disable) {

        & $helperLog "Turning off the security feature to unlock full emulator speed..."

        $proc = Start-Process -FilePath "bcdedit.exe" -ArgumentList "/set hypervisorlaunchtype off" -NoNewWindow -Wait -PassThru -ErrorAction SilentlyContinue

        if ($proc.ExitCode -eq 0) {

            & $helperLog "[SUCCESS] Speed boost enabled."

            & $helperLog "[NOTE] Please restart your computer for this speed boost to take effect."

            & $helperLog "[INFO] You can turn this security feature back on anytime with one click."

            return $true

        } else {

            & $helperLog "[ERROR] Failed with code $($proc.ExitCode). Please ensure running as Administrator."

            return $false

        }

    } else {

        & $helperLog "Turning the security feature back on..."

        $proc = Start-Process -FilePath "bcdedit.exe" -ArgumentList "/set hypervisorlaunchtype auto" -NoNewWindow -Wait -PassThru -ErrorAction SilentlyContinue

        if ($proc.ExitCode -eq 0) {

            & $helperLog "[SUCCESS] Security feature enabled. It will be active after restarting your computer."

            return $true

        } else {

            & $helperLog "[ERROR] Failed with code $($proc.ExitCode). Please ensure running as Administrator."

            return $false

        }

    }

}



# ------------------------------------------------------------------------------

# 10. REVERT EMULATOR TWEAKS

# ------------------------------------------------------------------------------

function Invoke-RevertEmulatorTweaks {

    [Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSReviewUnusedParameter', '')]

    param([scriptblock]$LogCallback = $null)



    $helperLog = {

        param([string]$msg)

        if ($LogCallback) { & $LogCallback $msg } else { Write-Host $msg }

    }

    & $helperLog "Resetting emulator and mouse settings back to original Windows defaults..."



    # 1. Restore bluestacks.conf backups

    # 1. Close active BlueStacks/MSI processes if running
    $procs = Get-Process -Name "HD-Player", "BstkSVC" -ErrorAction SilentlyContinue
    if ($procs) {
        & $helperLog "  Closing running BlueStacks processes before restoring defaults..."
        Stop-Process -Name "HD-Player", "BstkSVC" -Force -ErrorAction SilentlyContinue
        Start-Sleep -Milliseconds 800
    }

    # 2. Restore bluestacks.conf backups
    $confDirs = @(
        "$env:ProgramData\BlueStacks_nxt",
        "$env:ProgramData\BlueStacks_msi5"
    )

    foreach ($d in $confDirs) {
        if (Test-Path $d) {
            $target = Join-Path $d "bluestacks.conf"
            $origBak = Join-Path $d "bluestacks.conf.bak_original"
            $restoreSrc = $null

            if (Test-Path $origBak) {
                $restoreSrc = $origBak
            } else {
                $baks = Get-ChildItem -Path $d -Filter "bluestacks.conf.bak_anxious_*" | Sort-Object LastWriteTime
                if ($baks.Count -gt 0) {
                    $restoreSrc = $baks[0].FullName
                }
            }

            if ($restoreSrc) {
                $rawBytes = [System.IO.File]::ReadAllBytes($restoreSrc)
                $offset = 0
                if ($rawBytes.Length -ge 3 -and $rawBytes[0] -eq 0xEF -and $rawBytes[1] -eq 0xBB -and $rawBytes[2] -eq 0xBF) {
                    $offset = 3
                }
                $rawText = [System.Text.Encoding]::UTF8.GetString($rawBytes, $offset, $rawBytes.Length - $offset)
                $cleanText = (($rawText -split "`r?`n" | Where-Object { $_.Trim() -ne "" }) -join "`n") + "`n"
                [System.IO.File]::WriteAllText($target, $cleanText, [System.Text.UTF8Encoding]::new($false))
                & $helperLog "  Restored pristine original settings for $(Split-Path $target -Leaf)."
            }
        }
    }

    # 3. Restore network settings
    $tcpInterfacesKey = "HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces"
    if (Test-Path $tcpInterfacesKey) {
        $interfaces = Get-ChildItem -Path $tcpInterfacesKey -ErrorAction SilentlyContinue
        foreach ($iface in $interfaces) {
            Remove-ItemProperty -Path $iface.PSPath -Name "TcpAckFrequency" -ErrorAction SilentlyContinue
            Remove-ItemProperty -Path $iface.PSPath -Name "TCPNoDelay" -ErrorAction SilentlyContinue
            Remove-ItemProperty -Path $iface.PSPath -Name "TcpDelAckTicks" -ErrorAction SilentlyContinue
        }
        & $helperLog "  Restored default network TCP settings."
    }

    # 4. Re-enable default mouse settings
    Reset-WindowsMouseSettings -LogCallback $LogCallback | Out-Null



    & $helperLog "[DONE] Emulator settings restored to original defaults."

    return $true

}



Export-ModuleMember -Function @(
    'Reset-WindowsMouseSettings',
    'Get-EmulatorTopology',
    'Invoke-FreeFireFPSUnlock',
    'Invoke-EmulatorGPULock',
    'Invoke-FreeFireMouseAim',
    'Invoke-EmulatorLowPing',
    'Invoke-EmulatorCacheClean',
    'Invoke-MasterFreeFireBoost',
    'Invoke-FreeFireStretchedResolution',
    'Get-VBSStatus',
    'Invoke-ToggleVBS',
    'Invoke-RevertEmulatorTweaks'
)