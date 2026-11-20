# ==============================================================================
# Optimizer by Raghav - Verified Web Bootstrapper
# Author: Raghav Goyal
# ==============================================================================

[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 -bor [Net.SecurityProtocolType]::Tls13

$RepoSlug = "raghavatgit/AnxiouslyOptimized"
$Branch = "main"

# 1. Administrator Elevation Check
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host "[Optimizer by Raghav] Requesting Administrator privileges..." -ForegroundColor Cyan
    $launchCmd = "irm https://raw.githubusercontent.com/$RepoSlug/$Branch/run.ps1 | iex"
    Start-Process powershell.exe -ArgumentList "-NoProfile -ExecutionPolicy Bypass -Command `"$launchCmd`"" -Verb RunAs
    exit
}

# 2. Setup Staging Directory
$targetDir = Join-Path $env:TEMP "OptimizerByRaghav"
if (-not (Test-Path $targetDir)) {
    New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
}

Get-Process -Name "OptimizerByRaghav" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 300

$exePath = Join-Path $targetDir "OptimizerByRaghav.exe"
$shaPath = Join-Path $targetDir "OptimizerByRaghav.sha256"
$cacheBuster = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
$exeUrl = "https://raw.githubusercontent.com/$RepoSlug/$Branch/dist/OptimizerByRaghav.exe?t=$cacheBuster"
$shaUrl = "https://raw.githubusercontent.com/$RepoSlug/$Branch/dist/OptimizerByRaghav.sha256?t=$cacheBuster"

Write-Host "[Optimizer by Raghav] Downloading native Rust executable + SHA256 checksum..." -ForegroundColor Cyan

Invoke-WebRequest -Uri $exeUrl -OutFile $exePath -UseBasicParsing
Invoke-WebRequest -Uri $shaUrl -OutFile $shaPath -UseBasicParsing

# 3. Verify SHA256 Integrity
$expected = ((Get-Content $shaPath -Raw).Trim() -split '\s+')[0].ToLowerInvariant()
$actual = (Get-FileHash -Path $exePath -Algorithm SHA256).Hash.ToLowerInvariant()

if ([string]::IsNullOrWhiteSpace($expected) -or ($expected -ne $actual)) {
    Remove-Item $exePath -Force -ErrorAction SilentlyContinue
    Write-Host "[Optimizer by Raghav] ERROR: SHA256 verification failed! Expected '$expected', got '$actual'." -ForegroundColor Red
    exit 1
}

Write-Host "[Optimizer by Raghav] SHA256 verified ($actual). Launching..." -ForegroundColor Green
Start-Process -FilePath $exePath -WorkingDirectory $targetDir -Verb RunAs
