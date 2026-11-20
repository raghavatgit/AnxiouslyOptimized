<#
.SYNOPSIS
Builds both the standalone executable (dist\OptimizerByRaghav.exe) and the native
Windows Setup Installer (dist\OptimizerByRaghav-Setup.exe), generates .sha256 files,
and validates PE VersionInfo.
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $RepoRoot

function Copy-WithRunningFallback {
    param([string]$Source, [string]$Destination)
    try {
        Copy-Item -Path $Source -Destination $Destination -Force -ErrorAction Stop
    }
    catch {
        $Dir = Split-Path -Parent $Destination
        $Name = Split-Path -Leaf $Destination
        $TempBak = Join-Path $Dir ".$Name.running.tmp"
        Remove-Item -Path $TempBak -Force -ErrorAction SilentlyContinue
        Move-Item -Path $Destination -Destination $TempBak -Force -ErrorAction SilentlyContinue
        Copy-Item -Path $Source -Destination $Destination -Force -ErrorAction Stop
        Remove-Item -Path $TempBak -Force -ErrorAction SilentlyContinue
    }
}

try {
    $CargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
    if (Test-Path $CargoBin) {
        $env:PATH = "$CargoBin;$env:PATH"
    }

    $MsvcBin = 'D:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.51.36231\bin\Hostx64\x64'
    $MsvcLib = 'D:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.51.36231\lib\x64'
    $SdkBin  = 'D:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64'
    $SdkUm   = 'D:\Program Files (x86)\Windows Kits\10\Lib\10.0.26100.0\um\x64'
    $SdkUcrt = 'D:\Program Files (x86)\Windows Kits\10\Lib\10.0.26100.0\ucrt\x64'
    if (Test-Path $MsvcBin) {
        $env:PATH = "$MsvcBin;$SdkBin;$env:PATH"
        $env:LIB  = "$MsvcLib;$SdkUm;$SdkUcrt"
        $env:RC_PATH = Join-Path $SdkBin 'rc.exe'
    }

    if (-not (Get-Command cargo.exe -ErrorAction SilentlyContinue)) {
        throw 'Rust toolchain (cargo.exe) not found. Install rustup or ensure ~/.cargo/bin is on PATH.'
    }

    $DistDir = Join-Path $RepoRoot 'dist'
    if (-not (Test-Path $DistDir)) {
        New-Item -ItemType Directory -Path $DistDir -Force | Out-Null
    }

    # 1. Build Standalone App Binary (OptimizerByRaghav.exe)
    Write-Host '[Optimizer by Raghav] Step 1/2: Building standalone app binary (OptimizerByRaghav.exe)...' -ForegroundColor Cyan
    & cargo build --release --bin OptimizerByRaghav
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build --release --bin OptimizerByRaghav failed with exit code $LASTEXITCODE"
    }

    $TargetExe = Join-Path $RepoRoot 'target\release\OptimizerByRaghav.exe'
    $DistExe   = Join-Path $DistDir 'OptimizerByRaghav.exe'
    Copy-WithRunningFallback -Source $TargetExe -Destination $DistExe

    $AppHash = (Get-FileHash -Path $DistExe -Algorithm SHA256).Hash.ToLowerInvariant()
    "$AppHash  OptimizerByRaghav.exe" | Set-Content -Path (Join-Path $DistDir 'OptimizerByRaghav.sha256') -Encoding ascii

    # 2. Build Native Windows Setup Installer (OptimizerByRaghav-Setup.exe) embedding dist\OptimizerByRaghav.exe
    Write-Host '[Optimizer by Raghav] Step 2/2: Building native Windows Setup Installer (OptimizerByRaghav-Setup.exe)...' -ForegroundColor Cyan
    & cargo build --release --bin OptimizerByRaghav-Setup
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build --release --bin OptimizerByRaghav-Setup failed with exit code $LASTEXITCODE"
    }

    $TargetSetup = Join-Path $RepoRoot 'target\release\OptimizerByRaghav-Setup.exe'
    $DistSetup   = Join-Path $DistDir 'OptimizerByRaghav-Setup.exe'
    Copy-WithRunningFallback -Source $TargetSetup -Destination $DistSetup

    $SetupHash = (Get-FileHash -Path $DistSetup -Algorithm SHA256).Hash.ToLowerInvariant()
    "$SetupHash  OptimizerByRaghav-Setup.exe" | Set-Content -Path (Join-Path $DistDir 'OptimizerByRaghav-Setup.sha256') -Encoding ascii

    $AppItem   = Get-Item $DistExe
    $SetupItem = Get-Item $DistSetup
    $Ver       = $AppItem.VersionInfo

    Write-Host "[Optimizer by Raghav] App Exe   : $DistExe ($([math]::Round($AppItem.Length / 1MB, 2)) MB) | SHA256: $AppHash" -ForegroundColor Green
    Write-Host "[Optimizer by Raghav] Setup Exe : $DistSetup ($([math]::Round($SetupItem.Length / 1MB, 2)) MB) | SHA256: $SetupHash" -ForegroundColor Green
    Write-Host "[Optimizer by Raghav] PE Info   : Product='$($Ver.ProductName)' | FileVer='$($Ver.FileVersion)'" -ForegroundColor Gray

    if ($Ver.ProductName -ne 'Optimizer by Raghav') {
        throw "Unexpected PE ProductName: $($Ver.ProductName)"
    }
}
finally {
    Pop-Location
}
