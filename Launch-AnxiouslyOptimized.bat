@echo off
:: ==============================================================================
::  ANXIOUSLYOPTIMIZED - ONE-CLICK ADMINISTRATOR LAUNCHER
:: ==============================================================================
title Launching AnxiouslyOptimized...
cd /d "%~dp0"

if exist "%~dp0dist\AnxiouslyOptimized.exe" (
    powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "Start-Process '%~dp0dist\AnxiouslyOptimized.exe' -WorkingDirectory '%~dp0' -Verb RunAs"
    exit /b 0
)

powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "Start-Process powershell.exe -ArgumentList '-NoProfile -ExecutionPolicy Bypass -File \"%~dp0AnxiouslyOptimized.ps1\"' -WorkingDirectory '%~dp0' -Verb RunAs"
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [ERROR] Failed to elevate process. Please right-click this file and select 'Run as Administrator'.
    pause
)
