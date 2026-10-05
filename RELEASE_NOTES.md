# AnxiouslyOptimized v1.1.0

**Valorant Esports Optimizer Engine & Minimalist UI Release**

AnxiouslyOptimized v1.1.0 introduces a dedicated, commercial-grade **Valorant Esports Optimizer Engine**, designed strictly for competitive players and 100% Riot Vanguard compliance. This update also refactors the UI with a clean, minimal typography system, removes all hardcoded placeholder metrics in favor of live kernel telemetry, and improves startup responsiveness.

---

## What's New in v1.1.0

### 🎯 Valorant Esports Optimizer Engine (100% Vanguard Compliant)
- **Zero-Risk Architecture**: Modifies only verified user-level config files (`GameUserSettings.ini`), OS network QoS, and Win32 process priorities. No DLL injection, memory hooking, or game binary tampering.
- **Three Tailored Competitive Presets**:
  - **Competitive Potato**: 70% 3D render scaling, low LOD, disabled letterboxing, and aggressive P-Core affinity for budget rigs, iGPUs, and older CPUs.
  - **Tournament 240Hz**: Native 1080p clarity, low-spec shaders, AMD 3D V-Cache (CCD0) / Intel P-Core lock, and DSCP 46 high-priority network packets.
  - **Laptop Balanced**: 85% dynamic render scale, thermals-focused profile to eliminate laptop CPU/GPU throttling during extended competitive matches.
- **Dynamic Core Affinity Daemon**: Detects `VALORANT-Win64-Shipping.exe` on launch and dynamically assigns it to high-performance cores (avoiding Intel Gracemont E-cores and AMD non-cache CCDs), while demoting Riot auxiliary processes.
- **Shader Pipeline & Log Purger**: 1-click clearance of DirectX, NVIDIA, AMD, and Unreal Engine shader caches, logs, and crash dumps to eliminate 99% loading stalls.
- **Real-Time Riot Cluster Ping Probe**: Concurrent ICMP latency benchmark across official Riot Games server clusters (NA, EU, AP, KR, BR).

### ⚡ True Live Telemetry & Zero Hardcoded Metrics
- **Native Kernel Timer Telemetry**: Replaced static placeholders with direct `NtQueryTimerResolution` Win32 calls, measuring true real-time scheduler resolution down to 0.1ms precision.
- **Instantaneous Gauge Realization**: Live CPU, RAM, and SSD telemetry now queries and renders synchronously on launch with zero delay or empty placeholders.
- **Dynamic Hardware Fallbacks**: System RAM, GPU, and Storage queries fall back to native Win32 `GlobalMemoryStatusEx` and `DriveInfo` rather than fixed constants.

### 🎨 Clean, Minimalist Typography & Interface
- **Anti-AI Typography**: Standardized on native Windows typography (`Segoe UI`, `Consolas`), completely eradicating `Bahnschrift` and web CSS font artifacts.
- **Strict Modular Scale**: Replaced 17 arbitrary fractional font sizes (`10.5`, `11.5`, `12.5`, etc.) with a clean 10/11/12/13/14/16/18px typographic scale.
- **Distilled UI Copy**: Reduced wordy descriptions and card text across all 9 navigation tabs for a clean, scan-friendly, mission-critical dashboard.
- **Glyph & Encoding Polish**: Corrected unicode bullet references to standard XML character entities (`&#x2022;`).

---

# AnxiouslyOptimized v1.0.0

**First public release.**

AnxiouslyOptimized is a Windows gaming optimizer, debloater, and system cleaner built for competitive PC gamers. It targets real, measurable latency sources - scheduler timers, shader caches, network stack, and bloatware - not placebo tweaks.

---

## What's Included

- **Game Mode Daemon** - Activates 1ms scheduling timer, applies CPU affinity and priority boosts while a game is running; restores everything on exit. P-core aware on hybrid Intel CPUs.
- **Tweak Engine** - Registry and PowerShell-backed tweaks for network stack (autotuning, RSS, ECN), GPU scheduling, HPET, memory management, and Windows services. Full undo via transaction journal.
- **Deep System Cleaner** - Purges DirectX/GPU shader caches, Windows Update leftovers, Chromium code caches, crash dumps, temp files, and thumbnail databases. Skips locked files when browsers are open.
- **Windows Debloater** - Scans and removes pre-installed UWP bloatware via a config-driven catalog (`debloat_blacklist.json`). Does not touch Xbox Game Bar or Gaming App by default.
- **Runtime Hub** - Installs Visual C++ Redistributables, DirectX Runtime, .NET 8, 7-Zip, RTSS, OBS, Steam, and Discord via winget with direct CDN fallback. SHA256 verified for fixed-URL packages.
- **Network Optimizer** - Sets Cloudflare (1.1.1.1) or Google (8.8.8.8) DNS on the active adapter; restores original DNS on undo.
- **Safety System** - Every tweak is journaled with a timestamp-GUID key. Emergency `.bat` rollback is generated per session and covers all registry value kinds including ExpandString and Binary.

---

## Reliability Fixes in This Release (34 defects closed)

### Critical
- Fixed race condition in `TransactionJournal` (concurrent thread write without lock)
- Fixed JournalId collision (now timestamp + GUID fragment)
- Fixed rollback dropping ExpandString, MultiString, and Binary registry values
- Fixed emergency `.bat` using shorthand hive names (`HKCU`) that `reg.exe` rejects
- Fixed Windows Update cache purge running during active staged downloads

### High
- Fixed Game Mode daemon PID recycling false-positive (game gone, new process reuses PID)
- Fixed 1ms timer not released on crash (three global exception handlers now call `TimeEndPeriod`)
- Fixed DNS netsh command exit code never checked; failure now logged and returned
- Fixed packages marked Installed on installer exit code 1 (UAC cancel) or any non-zero code
- Fixed PowerShell process orphan on tweak apply timeout (now killed after 12s)
- Fixed `Get-AppxPackage` stdout deadlock on full pipe buffer (async ReadToEnd)
- Fixed backup directory using `BaseDirectory` (portable EXE path); now `%LOCALAPPDATA%`
- Fixed `RunToolSilent` deadlock on stderr fill (async ReadToEnd before WaitForExit)

### Medium
- Fixed network autotuning sentinel so rollback detects manual-applied vs Windows default
- Fixed P-core count returning logical thread count on hybrid CPUs (WMI `NumberOfCores`)
- Fixed tweak result parsing broken by colon in registry value names (switched to `|` delimiter)
- Fixed registry backup covering only 4 keys; now covers 12 including Policies and CLSID
- Fixed `WebClient.DownloadFile` (no timeout, deprecated TLS); replaced with PowerShell subprocess
- Fixed `powercfg -duplicatescheme` running even when power scheme already exists
- Fixed rollback marking `IsRolledBack = true` even when zero steps were actually reverted
- Fixed SSD detection relying entirely on model name string heuristics; now uses WMI `MediaType`

### Low
- `debloat_blacklist.json` embedded in EXE and read at runtime (was ignored)
- `debloat_whitelist.json` embedded in EXE
- SHA256 verification for DirectX CDN installer (`2cf71d098c608c56e07f4655855a886c3102553f648df88458df616b26fd612f`)
- Emergency `.bat` filename includes session JournalId; no silent overwrite between runs
- Single-instance mutex prevents two concurrent processes
- Global WPF, AppDomain, and Task exception handlers all release the 1ms timer

---

## System Requirements

- Windows 10 21H2 or Windows 11 (any version)
- .NET Framework 4.8 (included in Windows 10 1903+)
- Run as Administrator (required for registry and service operations)

---

## Known Limitations

- `MainWindow.xaml.cs` is a large single file; ViewModel refactor is planned for v1.1
- Self-update check not yet implemented
- `aka.ms` redirected installers (VC++ Redist, .NET Runtime) are not SHA256 verified because Microsoft updates the binary without changing the redirect URL
