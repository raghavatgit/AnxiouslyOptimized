# ==============================================================================
# AnxiouslyOptimized - Zero-Install Web Launcher
# Author: Raghav Goyal (raghavatgit)
# ==============================================================================

[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 -bor [Net.SecurityProtocolType]::Tls13

# 1. Administrator Elevation Check
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host "[AnxiouslyOptimized] Requesting Administrator privileges..." -ForegroundColor Cyan
    $launchCmd = "irm https://raw.githubusercontent.com/raghavatgit/AnxiouslyOptimized/main/run.ps1 | iex"
    Start-Process powershell.exe -ArgumentList "-NoProfile -ExecutionPolicy Bypass -Command `"$launchCmd`"" -Verb RunAs
    exit
}

# 2. Setup Staging Directory
$targetDir = Join-Path $env:TEMP "AnxiouslyOptimized"
if (-not (Test-Path $targetDir)) {
    New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
}

$exePath = Join-Path $targetDir "AnxiouslyOptimized.exe"
$downloadUrl = "https://raw.githubusercontent.com/raghavatgit/AnxiouslyOptimized/main/dist/AnxiouslyOptimized.exe"

# 3. Stream Executable Binary
Write-Host "[AnxiouslyOptimized] Initializing zero-install launcher..." -ForegroundColor Cyan
Write-Host "[AnxiouslyOptimized] Downloading standalone Direct3D binary..." -ForegroundColor Gray

try {
    $wc = New-Object System.Net.WebClient
    $wc.Headers.Add("User-Agent", "AnxiouslyOptimized-WebLauncher")
    $wc.DownloadFile($downloadUrl, $exePath)
} catch {
    Write-Host "[AnxiouslyOptimized] Primary download failed. Retrying with Invoke-WebRequest..." -ForegroundColor Yellow
    Invoke-WebRequest -Uri $downloadUrl -OutFile $exePath -UseBasicParsing
}

# 4. Verify Integrity and Launch
if ((Test-Path $exePath) -and ((Get-Item $exePath).Length -gt 100000)) {
    Write-Host "[AnxiouslyOptimized] Launching native application..." -ForegroundColor Green
    Start-Process -FilePath $exePath -WorkingDirectory $targetDir -Verb RunAs
} else {
    Write-Host "[AnxiouslyOptimized] Error: Binary download failed or corrupt." -ForegroundColor Red
}
