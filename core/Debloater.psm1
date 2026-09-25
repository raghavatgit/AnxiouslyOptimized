# ==============================================================================
#  ANXIOUSLYOPTIMIZED - SAFE DEBLOAT & APPX PACKAGE MANAGER
#  Non-destructive AppX provision remover and telemetry service neutralizer
# ==============================================================================

[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidUsingWriteHost', '')]
[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSUseSingularNouns', '')]
[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidUsingEmptyCatchBlock', '')]
param()

$script:ConfigDir = Join-Path $PSScriptRoot "..\config"

function Get-DebloatLists {
    $whitelistPath = Join-Path $script:ConfigDir "debloat_whitelist.json"
    $blacklistPath = Join-Path $script:ConfigDir "debloat_blacklist.json"
    
    $whitelist = @()
    if (Test-Path $whitelistPath) {
        $whitelist = Get-Content $whitelistPath -Raw | ConvertFrom-Json
    }
    
    $blacklist = @()
    if (Test-Path $blacklistPath) {
        $blacklist = Get-Content $blacklistPath -Raw | ConvertFrom-Json
    }
    
    return @{ Whitelist = $whitelist; Blacklist = $blacklist }
}

function Get-InstalledBloatware {
    $lists = Get-DebloatLists
    $isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    $installedApps = if ($isAdmin) {
        Get-AppxPackage -AllUsers -ErrorAction SilentlyContinue
    } else {
        Get-AppxPackage -ErrorAction SilentlyContinue
    }
    
    $found = @()
    foreach ($item in $lists.Blacklist) {
        # Check if matched package is currently installed
        $match = $installedApps | Where-Object { $_.Name -like "*$($item.name)*" }
        if ($match) {
            # Double check against whitelist
            $isWhitelisted = $false
            foreach ($white in $lists.Whitelist) {
                if ($match.Name -like "*$white*") {
                    $isWhitelisted = $true
                    break
                }
            }
            
            if (!$isWhitelisted) {
                $found += [PSCustomObject]@{
                    Name        = $item.name
                    DisplayName = $item.displayName
                    Description = $item.description
                    PackageFullName = $match[0].PackageFullName
                }
            }
        }
    }
    return $found
}

function Invoke-SafeDebloat {
    param(
        [string[]]$PackageNames,
        [scriptblock]$LogCallback
    )
    
    $lists = Get-DebloatLists
    $isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    $installedApps = if ($isAdmin) {
        Get-AppxPackage -AllUsers -ErrorAction SilentlyContinue
    } else {
        Get-AppxPackage -ErrorAction SilentlyContinue
    }
    $provisionedApps = if ($isAdmin) {
        Get-AppxProvisionedPackage -Online -ErrorAction SilentlyContinue
    } else {
        @()
    }
    
    $targets = @()
    if ($PackageNames -and $PackageNames.Count -gt 0) {
        $targets = $lists.Blacklist | Where-Object { $_.name -in $PackageNames }
        foreach ($pkgName in $PackageNames) {
            if (!($targets | Where-Object { $_.name -eq $pkgName })) {
                $targets += [PSCustomObject]@{
                    name = $pkgName
                    displayName = $pkgName
                    description = "Selected Windows App Package"
                }
            }
        }
    } else {
        $targets = $lists.Blacklist
    }
    
    $removedCount = 0
    
    foreach ($target in $targets) {
        $name = $target.name
        $disp = if ($target.displayName) { $target.displayName } else { $name }
        
        # Hard whitelist check - never remove essential system apps
        $isSafe = $true
        foreach ($white in $lists.Whitelist) {
            if ($name -like "*$white*") { $isSafe = $false; break }
        }
        if (!$isSafe) { continue }
        
        # Check per-user AppX
        $userPkg = $installedApps | Where-Object { $_.Name -like "*$name*" }
        $didRemove = $false
        if ($userPkg) {
            if ($LogCallback) { & $LogCallback "Removing app: $disp ($name)..." }
            else { Write-Host "  Removing: $disp..." -ForegroundColor Yellow }
            
            foreach ($pkg in $userPkg) {
                try {
                    if ($isAdmin) {
                        Remove-AppxPackage -Package $pkg.PackageFullName -AllUsers -ErrorAction Stop
                    } else {
                        Remove-AppxPackage -Package $pkg.PackageFullName -ErrorAction Stop
                    }
                    $didRemove = $true
                } catch {
                    try {
                        Remove-AppxPackage -Package $pkg.PackageFullName -ErrorAction SilentlyContinue
                        $didRemove = $true
                    } catch { }
                }
            }
        }
        
        # Check provisioned AppX (stops package from reinstalling on new users or feature updates)
        if ($provisionedApps) {
            $provPkgs = $provisionedApps | Where-Object { $_.DisplayName -like "*$name*" -or $_.PackageName -like "*$name*" }
            foreach ($provPkg in $provPkgs) {
                try {
                    Remove-AppxProvisionedPackage -Online -PackageName $provPkg.PackageName -ErrorAction SilentlyContinue | Out-Null
                    $didRemove = $true
                } catch { }
            }
        }

        if ($didRemove) {
            $removedCount++
        }
    }
    
    return $removedCount
}

Export-ModuleMember -Function Get-InstalledBloatware, Invoke-SafeDebloat, Get-DebloatLists
