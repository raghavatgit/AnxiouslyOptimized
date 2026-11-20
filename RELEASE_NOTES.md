# Release Notes — Optimizer by Raghav

## v2.1.0 — Native Windows Setup Installer, GitHub Update Checker, 240 FPS UI/UX & Emulator Engine Hardening

### Highlights
- **Native Windows Setup Installer (`dist\OptimizerByRaghav-Setup.exe`) & System Integration**:
  - Dedicated standalone GUI & CLI (`--silent`, `--uninstall`) Windows Setup Wizard that embeds `OptimizerByRaghav.exe`, installs into `%ProgramFiles%\Optimizer by Raghav`, generates Desktop & Start Menu shortcuts, and registers in Windows **Settings > Installed Apps** (`HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\OptimizerByRaghav`).
  - Also accessible directly inside the app via **Settings > Windows Desktop Installation** or via `OptimizerByRaghav.exe --install` / `--uninstall`.
- **Live GitHub Release Update Checker (`src/engine/updater.rs`)**:
  - Top-bar **Check Updates** button + modal dialog and Settings card that query the GitHub Releases API (`https://api.github.com/repos/raghavatgit/AnxiouslyOptimized/releases/latest`) with raw `version.json` fallback.
  - Compares semantic versions (`v2.1.0`), displays release notes, and provides one-click **Download & Install Update Now** or direct GitHub Release page opening.
- **Swifter 240 FPS UI/UX Architecture**:
  - **Virtualized Uninstaller Table**: Replaced O(N) per-row allocation in Geek Uninstaller with `egui::ScrollArea::vertical().show_rows(...)` viewport virtualization and index filtering (zero per-frame `InstalledApp` struct cloning).
  - **Zero Per-Frame Log Mutex Cloning**: Activity Log dock now reads directly under lock without cloning the entire `Vec<LogEntry>` on every frame, auto-scrolling only when new entries arrive.
  - **Refined Obsidian-Slate Theme & Grouped Sidebar**: Added section headers (`OVERVIEW`, `OPTIMIZE & CLEAN`, `GAMING & SAFETY`), glowing active accent bar indicator, 110ms ultra-responsive transitions, and keyboard shortcuts (`Ctrl+1..9` tab jump, `Ctrl+R` Quick Mem Reduct, `F5` Re-Scan Hardware).
- **7 Real-World Android Emulator Engine Fixes (`src/engine/emulator.rs`)**:
  - Fixed malformed `enable_ root_access` key stripping, recognized MSI/BlueStacks built-in `"rogt"` ROG 2 profile code, stopped `BstkSVC.exe` before patching `bluestacks.conf` to prevent overwrite on daemon exit, added real MSI 5.12 / BlueStacks 5 keys (`astc_decoding_mode="hardware"`, `bst.prefer_dedicated_gpu="1"`, `bst.enable_mem_dedup="1"`, `bst.enable_programmatic_ads="0"`, `bst.feature.bluestacksX="0"`), removed invalid user-mode `IoPriority=3` from IFEO `PerfOptions`, queried `HKLM\SOFTWARE\BlueStacks_msi5` for custom `InstallDir`/`UserDefinedDir`, and added `"basicSettings.highFp": true` for LDPlayer 9 240 FPS mode.

---

## v2.0.0 — Pure Rust Native System Identifier & Optimizer

Complete clean-slate rewrite from legacy C#/PowerShell prototypes to a single-binary **Rust** (`egui` / `eframe`) Windows desktop application named **Optimizer by Raghav**.

### Architectural Overhaul & Bug Fixes
- **Eliminated Legacy Fake/Stub Code**: Removed 79 duplicate stub files, committed personal `.reg` registry dumps, stale PowerShell prototypes, static hardcoded SVG sparklines, and fabricated UI labels.
- **Accurate Whole-System Discovery**:
  - Replaced buggy 32-bit WMI `AdapterRAM` (capped at 4 GB) and iGPU-first truncation with native **DXGI 1.6** enumeration across all adapters + WMI PNP/PCI matching.
  - Replaced integer `Convert.ToInt32(Win32_DiskDrive.MediaType)` crash with `MSFT_PhysicalDisk` `MediaType` / `BusType` / `HealthStatus`.
  - Added full SMBIOS DDR5/LPDDR3/4/5 per-DIMM slot identification, Win32 multi-monitor detection (`EnumDisplayDevicesW`), hybrid P-Core/E-Core & X3D CCD0 topology, TPM/SecureBoot/VBS, and primary physical NIC detection.
- **Safety & Transaction Integrity**:
  - Replaced lossy JSON registry rollback and `-1` DWORD sentinel collisions with strongly-typed `TypedRegValue` (`Dword`, `Qword`, `String`, `ExpandString`, `MultiString`, `Binary`) and strict LIFO rollback under `KEY_WOW64_64KEY`.
  - GPU MSI-mode now filters exclusively to Display adapter ClassGUID `{4d36e968-e325-11ce-bfc1-08002be10318}` instead of toggling all PCI devices.
  - NTFS last-access check masks bit-31 (`0x80000002` system-managed disabled) accurately.
  - Deep Cleaner enforces reparse-point skipping, strict wildcard filtering (`thumbcache_*.db`), and open browser skips.
  - Kernel memory cleaner now implements the full 6-region **Mem Reduct** architecture (`MemoryEmptyWorkingSets`, `SetSystemFileCacheSize`, `MemoryFlushModifiedList`, `MemoryPurgeStandbyList`, `MemoryPurgeLowPriorityStandbyList`, `SystemCombinePhysicalMemoryInformation`) with live Physical/Commit/Cache gauges and Auto-Reduct threshold/interval automation.
  - Added full **Geek.exe Uninstaller & Force-Cleaner** engine covering Win32 (64-bit, 32-bit WOW64, HKCU) and Store/UWP programs, automatic post-uninstall leftover scanning, Force Removal of orphaned folders/registry keys, and one-click Regedit jump.
  - Added **10 Extreme Windows 11 & eSports Tweaks** (Copilot/Recall off, Windowed Games DXGI Flip-Model + VRR, Widgets service block, Compact Explorer + Gallery removal, VBS/HVCI disable, `Win32PrioritySeparation=0x26`, `SvcHostSplitThresholdInKB` consolidation, `MenuShowDelay=0`, UWP background app block, CPU power throttling disable) + **Extreme Win11 & eSports** preset.
  - Expanded **Extreme Android Emulator Lab** (BlueStacks 5 / MSI App Player 5 / LDPlayer 9 240 FPS + ROG Phone 5 `ASUS_I005DA` + ASTC hardware decoding + 4GB+ RAM scaling + NowBux ad-block + Kernel IFEO `PerfOptions` permanent High CPU/IO priority lock + HVCI/Hypervisor toggle) and **14-category Extreme PC Cleaner** (including Prefetch, Opera GX/Vivaldi/Discord/Spotify/Steam/Epic caches, CBS/DISM logs, Font Cache, `SHEmptyRecycleBinW`, and `wevtutil` Event Log purge).
  - VALORANT GameUserSettings patching writes atomically via `MoveFileExW` without UTF-8 BOM and maintains 100% Vanguard process separation.
  - Concurrent Win32 `IcmpSendEcho` DNS benchmark treats `<1 ms` (`0 ms`) fiber links validly and fixes packet-loss precedence.
