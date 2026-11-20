@echo off
setlocal
title Optimizer by Raghav
set "ROOT=%~dp0"
set "EXE=%ROOT%dist\OptimizerByRaghav.exe"

if not exist "%EXE%" (
    echo [Optimizer by Raghav] Release binary not found at "%EXE%".
    echo [Optimizer by Raghav] Building native Rust release executable...
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%ROOT%build\Build-Release.ps1"
    if errorlevel 1 (
        echo [Optimizer by Raghav] Build failed. Press any key to exit.
        pause >nul
        exit /b 1
    )
)

echo [Optimizer by Raghav] Launching "%EXE%"...
powershell.exe -NoProfile -Command "Start-Process -FilePath '%EXE%' -Verb RunAs"
endlocal
