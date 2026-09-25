<p align="center">
  <img src="assets/logo.png" width="128" height="128" alt="AnxiouslyOptimized Logo" />
</p>

# AnxiouslyOptimized - Easy PC Speed & Game Booster

> **Safe, Simple Windows 11 & Windows 10 Speed Optimizer & Unwanted App Cleaner**  
> *100% Safe Standard: Automatic Backups, Reversible Anytime, Never Breaks Windows.*

---

## Overview

**AnxiouslyOptimized** is a friendly, safe, and easy-to-use Windows optimization and cleanup tool made for **everyone** - everyday home and office users, PC gamers, and mobile emulator players.

No complicated settings, no confusing tech jargon, and no risky registry tricks. It cleans out unwanted junk apps, stops background data lag, smooths out game stutter, and gives your PC an instant boost with a single click.

---

## Ready-to-Use Programs (`dist/`)

The software is pre-packaged into single-click programs that work right out of the box:

| Program File | Who It Is For | What It Does |
| :--- | :--- | :--- |
| **`dist/AnxiouslyOptimized-Win11.exe`** | **Windows 11 Computers** | Instant classic right-click menu, removes taskbar widgets and news ads, cleans up File Explorer, and checks emulator speed. |
| **`dist/AnxiouslyOptimized-Win10.exe`** | **Windows 10 Computers** | Removes taskbar news and weather, hides Cortana button, turns off background activity history. |
| **`dist/AnxiouslyOptimized-Universal.exe`** | **All Windows Computers** | Automatically detects your Windows version and applies the right settings. |

> [!TIP]
> Just double-click any `.exe` in `dist/`, click **Yes** when Windows asks for permission, and the app opens right away. No setup or terminal needed.

---

## What It Can Do For Your PC

| Feature | Windows 11 | Windows 10 | Why It Helps |
| :--- | :---: | :---: | :--- |
| **Classic Right-Click Menu** | **YES** | *N/A* | Brings back the fast right-click menu without having to click "Show more options". |
| **Remove Taskbar Widgets & AI** | **YES** | *N/A* | Stops news feeds and background assistants from using your memory. |
| **Clean File Explorer** | **YES** | *N/A* | Stops Windows from showing recommendations and recent clutter in folders. |
| **Turn Off Taskbar News & Weather**| *N/A* | **YES** | Removes the taskbar news popup to save memory and stop lag. |
| **Hide Cortana Button** | *N/A* | **YES** | Hides the old assistant button from the taskbar. |
| **Turn Off Activity History** | *N/A* | **YES** | Stops Windows from recording everything you open into a history log. |
| **Stop Background Activity Tracking**| **YES** | **YES** | Pauses background diagnostic reports. Never breaks updates or the Microsoft Store. |
| **Instant Start Menu Search** | **YES** | **YES** | Stops searching Bing online so opening local apps and files is instant. |
| **Block Sponsored Game & App Ads** | **YES** | **YES** | Stops Windows from automatically downloading unwanted games like Candy Crush. |
| **Stop Game Stutter (Turn Off Xbox Recording)**| **YES**| **YES**| Stops background screen recording that causes frame drops and stutters while gaming. |
| **Smooth & Natural Mouse Aim** | **YES** | **YES** | Removes mouse speed jumping for consistent, natural aiming in games. |
| **Ultimate Performance Power Plan**| **YES** | **YES** | Keeps your processor ready at full speed without letting it slow down. |
| **Speed Up Graphics Card Processing**| **YES**| **YES**| Lets your graphics card manage its own memory for lower input lag in modern games. |
| **Faster Internet & Drive Loading** | **YES** | **YES** | Speeds up downloads and cuts game loading times on SSD drives. |
| **Unlock 240 FPS for Free Fire** | **YES** | **YES** | Unlocks high frame rates and sets ASUS ROG Phone 2 mode in BlueStacks 5 & MSI App Player. |
| **Force Best Graphics Card for Emulators**| **YES**| **YES**| Makes sure emulators always run on your dedicated NVIDIA/AMD graphics card. |
| **Smooth Mouse Aim for Drag Headshots**| **YES**| **YES**| Perfect upward mouse movement for Free Fire drag headshots without cursor jumping. |
| **Fix Game Ping & Stop Missed Hits**| **YES** | **YES** | Sends game bullets instantly over your internet so damage registers without delay. |

---

## Free Fire & Mobile Emulator Booster

AnxiouslyOptimized has special features built directly for **Free Fire** on **BlueStacks 5** and **MSI App Player**:

1. **Unlock 240 FPS & ROG Phone Mode**: Unlocks the "High FPS" option in Free Fire and lets you play smoothly at up to 240 FPS by setting your emulator profile to ASUS ROG Phone 2.
2. **Dedicated Graphics Card Lock**: Makes sure BlueStacks and Free Fire always run on your high-speed NVIDIA or AMD graphics card instead of slow built-in graphics.
3. **Smooth Mouse Aim (Drag Headshots)**: Turns off Windows mouse acceleration so upward drag headshots feel natural, smooth, and consistent.
4. **Fix Game Ping & Stop Missed Bullets**: Stops Windows from holding onto network packets, ensuring your shots hit right away and ping stays low.
5. **Clean Emulator Junk & Log Files**: Cleans out old error logs and temporary caches to fix the emulator getting stuck at 99% loading.

---

## Product Directory Structure

```
optimizefiles/
├── dist/                               # PRODUCTION STANDALONE BINARY
│   ├── AnxiouslyOptimized.exe          # Native C# WPF executable (Zero dependencies, Hardware accelerated)
│   └── data/theme.json                 # Persistent UI theme state (Obsidian, Cyberpunk, Emerald, Sapphire)
├── build/
│   └── Build-NativeApp.ps1             # Native MSBuild compilation pipeline producing dist/AnxiouslyOptimized.exe
├── src/                                # NATIVE C# APPLICATION SOURCE
│   ├── AnxiouslyOptimized.csproj       # Project configuration targeting .NET Framework 4.8
│   ├── App.xaml / App.xaml.cs          # WPF Application lifecycle & global theme dictionaries
│   ├── MainWindow.xaml / .cs           # Liquid glass WPF UI with 9 tabs, bento grid, and micro-animations
│   ├── Models/                         # Data contracts (TweakItem, BloatPackage, SystemSpecs, PresetConfig)
│   ├── Services/                       # High-performance native service layer (TweakService, SafetyService, etc.)
│   └── Assets/                         # Embedded configs, branding, logos, and vectorized icons
├── Launch-AnxiouslyOptimized.bat       # Double-click one-click Administrator launcher
├── AnxiouslyOptimized.ps1              # Master CLI engine with dynamic OS adaptation
├── README.md                           # Product documentation and commercial guide
├── config/
│   ├── tweaks_win11.json               # Windows 11 compliant optimizations
│   ├── tweaks_win10.json               # Windows 10 compliant optimizations
│   ├── presets.json                    # Consumer presets (Easy Mode, Gamer, Free Fire/Emulator)
│   ├── debloat_whitelist.json          # Universal safety whitelist
│   └── debloat_blacklist.json          # Safe consumer bloat catalog
├── core/
│   ├── Safety.psm1                     # System Restore, registry hive backups, hardware audit
│   ├── Engine.psm1                     # Dual-OS tweak dispatcher and status evaluator
│   ├── Debloater.psm1                  # Whitelist-guarded AppX package pruning
│   ├── Maintenance.psm1                # DISM /RestoreHealth, SFC /scannow, Component Cleanup
│   ├── Benchmark.psm1                  # Live CPU/RAM telemetry, power plan, timer resolution
│   └── Emulator.psm1                   # BlueStacks 5, MSI App Player & Free Fire hyper-engine
├── backups/                            # Reversible registry hive backups (.gitkeep preserved)
├── tests/
│   └── master_deep_audit.ps1           # 40-point automated deep validation test suite
└── assets/
    ├── gui.xaml                        # Liquid glass dark WPF interface definition
    ├── icon.ico                        # Multi-frame Windows icon (16x16 up to 256x256)
    ├── logo.png                        # Official branding card (512x512 rounded tile)
    └── logo_transparent.png            # Transparent PNG logo for overlays and media
```

---

## How to Run & Distribute

### Option 1: Standalone Native `.EXE` (Recommended for Customers)
Simply give your customer the `.exe` from the `dist/` directory:
* `dist\AnxiouslyOptimized.exe` (Native C# WPF, 275 KB, zero dependencies)

Customers double-click, click **Yes** on the Windows UAC prompt, and the app launches instantaneously (<50ms startup). No PowerShell, no terminal, no installations required.

### Option 2: One-Click Desktop Launcher (.BAT)
Customers can also double-click:
```bat
Launch-AnxiouslyOptimized.bat
```

### Option 3: Command Line / Scripting Automation
For advanced users, system administrators, or silent scripts:
```powershell
# Scan current settings:
.\AnxiouslyOptimized.ps1 -Scan

# Force Windows 11 scan:
.\AnxiouslyOptimized.ps1 -Scan -TargetOS Win11

# Force Windows 10 scan:
.\AnxiouslyOptimized.ps1 -Scan -TargetOS Win10

# Apply 1-Click Instant PC Boost:
.\AnxiouslyOptimized.ps1 -Preset Safe

# Apply Gaming Rig boost:
.\AnxiouslyOptimized.ps1 -Preset Gaming

# Apply Android Emulator boost:
.\AnxiouslyOptimized.ps1 -Preset Emulator

# Run Safe Debloater:
.\AnxiouslyOptimized.ps1 -Debloat

# Run System Repair (SFC + DISM):
.\AnxiouslyOptimized.ps1 -Repair

# Reclaim space & purge shader caches:
.\AnxiouslyOptimized.ps1 -CleanCache

# Undo all changes (100% factory revert):
.\AnxiouslyOptimized.ps1 -RevertAll
```

---

## Re-building the Native Executable
To recompile the retail `.exe` binary after code or UI adjustments:
```powershell
.\build\Build-NativeApp.ps1
```
The build pipeline compiles `src\AnxiouslyOptimized.csproj` using MSBuild in under 2 seconds and outputs `dist\AnxiouslyOptimized.exe`.
