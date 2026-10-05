@echo off
:: ==============================================================================
::  ANXIOUSLYOPTIMIZED - ONE-CLICK ADMINISTRATOR LAUNCHER
:: ==============================================================================
title Launching AnxiouslyOptimized...
cd /d "%~dp0"

:: 1. Check for compiled binary in dist\
if exist "%~dp0dist\AnxiouslyOptimized.exe" (
    powershell.exe -WindowStyle Hidden -NoProfile -ExecutionPolicy Bypass -Command "Start-Process '%~dp0dist\AnxiouslyOptimized.exe' -WorkingDirectory '%~dp0' -Verb RunAs"
    exit /b 0
)

:: 2. Check for compiled binary in root directory
if exist "%~dp0AnxiouslyOptimized.exe" (
    powershell.exe -WindowStyle Hidden -NoProfile -ExecutionPolicy Bypass -Command "Start-Process '%~dp0AnxiouslyOptimized.exe' -WorkingDirectory '%~dp0' -Verb RunAs"
    exit /b 0
)

:: 3. If binary not found, auto-download latest release binary from GitHub
echo [INFO] Compiled binary not found locally. Fetching latest release from GitHub...
powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "$ErrorActionPreference = 'Stop'; try { if (-not (Test-Path '%~dp0dist')) { New-Item -ItemType Directory -Path '%~dp0dist' -Force | Out-Null }; [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; $ts = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds(); $url = 'https://raw.githubusercontent.com/raghavatgit/AnxiouslyOptimized/main/dist/AnxiouslyOptimized.exe?t=' + $ts; Invoke-WebRequest -Uri $url -OutFile '%~dp0dist\AnxiouslyOptimized.exe' -UseBasicParsing; Start-Process '%~dp0dist\AnxiouslyOptimized.exe' -WorkingDirectory '%~dp0' -Verb RunAs; exit 0 } catch { exit 1 }"
if %ERRORLEVEL% EQU 0 (
    exit /b 0
)

:: 4. Fallback to PowerShell script with hidden console window if offline or download failed
if exist "%~dp0AnxiouslyOptimized.ps1" (
    powershell.exe -WindowStyle Hidden -NoProfile -ExecutionPolicy Bypass -Command "Start-Process powershell.exe -ArgumentList '-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File \"%~dp0AnxiouslyOptimized.ps1\"' -WorkingDirectory '%~dp0' -Verb RunAs"
    if %ERRORLEVEL% NEQ 0 (
        echo.
        echo [ERROR] Failed to elevate process. Please right-click this file and select 'Run as Administrator'.
        pause
    )
    exit /b 0
)

echo [ERROR] Neither AnxiouslyOptimized.exe nor AnxiouslyOptimized.ps1 was found.
pause
