# Optimizer by Raghav

**Native Pure-Rust Windows System Identifier & Performance Optimizer (`v2.1.0`)**  
Built with Rust, `egui` / `eframe` GPU acceleration, direct Win32/DXGI/WMI/SMBIOS kernel telemetry, zero fake buttons, native Windows Setup Installer, GitHub Update Checker, and atomic transaction rollback.

---

## Key Capabilities

- **Whole-System Identification (Read-Only Safe)**
  - **CPU**: Full hybrid P-Core / E-Core topology via Win32 `GetLogicalProcessorInformationEx`, AMD X3D L3 CCD0 detection, base/max clocks, socket, L1/L2/L3 cache totals.
  - **GPU & Displays**: Native `DXGI 1.6` 64-bit dedicated VRAM + shared system memory across multi-GPU laptops & desktops (`RTX`/`RX` discrete + integrated), WDDM driver version, HAGS status, PCI `MSISupported` audit, and live multi-monitor `EnumDisplayDevicesW` resolution + refresh rates (`Hz`).
  - **Memory**: Exact SMBIOS `Win32_PhysicalMemory` per-DIMM slot mapping (DDR2/DDR3/DDR4/DDR5/LPDDR3/LPDDR4/LPDDR5, configured vs rated MT/s, form factor, manufacturer) + live `GlobalMemoryStatusEx` / `GetPerformanceInfo`.
  - **Storage**: `MSFT_PhysicalDisk` media/bus/health identification with Win32 fallback + live NTFS/FAT32 volume capacities.
  - **Platform, Security & Network**: Real Windows SKU (no fabricated edition strings), UBR build, motherboard/BIOS/chassis/battery, pending reboot reasons, Secure Boot, TPM, HVCI/VBS (`bcdedit`), active power scheme, true `NtQueryTimerResolution`, physical vs virtual NIC classification, and installed gaming ecosystem detection.

- **100% Real, Functional Optimization & Safety Modules (Standalone `.exe` + Native Windows Setup Installer)**
  - **Native Setup Installer & GitHub Update Checker**:
    - `dist\OptimizerByRaghav-Setup.exe`: Standalone Windows Setup Wizard that installs into `%ProgramFiles%\Optimizer by Raghav`, creates Desktop & Start Menu shortcuts, and registers in Windows **Installed Apps** (`Uninstall\OptimizerByRaghav`).
    - **Top-Bar Update Button**: Queries GitHub Releases API (`raghavatgit/AnxiouslyOptimized`) with `version.json` fallback to notify when newly released versions are available and supports one-click download & launch.
  - **Swift 240 FPS UI/UX & Keyboard Shortcuts**:
    - Grouped navigation sidebar (`OVERVIEW`, `OPTIMIZE & CLEAN`, `GAMING & SAFETY`) with glowing active accent indicator, virtualized `show_rows` program table, zero per-frame mutex cloning, and hotkeys (`Ctrl+1..9` tabs, `Ctrl+R` Quick Mem Reduct, `F5` Re-Scan Hardware).
  - **Mem Reduct Multi-Region Kernel Memory Cleaner**: Cleans all 6 Windows kernel memory regions via `NtSetSystemInformation` & `SetSystemFileCacheSize` (**Working Set**, **System File Cache**, **Modified Page List**, **Standby List**, **Priority-0 Low-Priority Standby**, and **Combine Physical Memory Lists**) with live Physical/Commit/System-Cache gauges and background **Auto-Reduct** (>X% threshold & interval timer).
  - **Geek.exe Program Uninstaller & Leftover Force-Cleaner**: Indexes 64-bit, 32-bit WOW64, per-user HKCU, and Store/UWP programs with size calculation, standard uninstall + automatic deep leftover file/folder/registry sweep, **Force Removal** mode for broken uninstallers, and instant folder/`regedit.exe` jump.
  - **Extreme Windows 11 & eSports Optimizations (33 Tweaks + 4 Presets)**: Disables Copilot & AI Recall (`DisableAIDataAnalysis`), enables Windowed Game Flip-Model & VRR (`SwapEffectUpgradeEnable=1`), disables Windows 11 Widgets & WebExperience Host, removes Explorer Gallery/OneDrive sync ads + enables Compact Mode, disables VBS/HVCI hypervisor overhead, sets `Win32PrioritySeparation=0x26`, consolidates `SvcHostSplitThresholdInKB`, zeroes `MenuShowDelay`, disables UWP background execution, and restricts GPU MSI-mode strictly to Display adapters.
  - **14-Category Extreme PC Cleaner**: Reparse-point-safe cleaning across Windows/User Temp, Prefetch (`*.pf`), DirectX/NVIDIA/AMD/Intel Shaders, Explorer `thumbcache_*.db`, Delivery Optimization, Windows Update downloads, Crash Dumps/WER/LiveKernelReports, 6 Browsers (Chrome/Edge/Brave/Opera GX/Vivaldi/Firefox), Discord/Spotify, Steam/Epic/Riot caches, CBS/DISM/Panther logs, Font Cache, Recycle Bin (`SHEmptyRecycleBinW`), Event Viewer (`.evtx`) channels, and Game/Emulator logs.
  - **Extreme Android Emulator & VALORANT Lab**: BlueStacks 5 / MSI App Player 5.12 / LDPlayer 9 multi-instance 240 FPS + ROG Phone (`rogt` / `ASUS_I005DA`) + ASTC hardware decoding (`astc_decoding_mode="hardware"`) + memory deduplication (`bst.enable_mem_dedup="1"`) + dedicated GPU lock (`bst.prefer_dedicated_gpu="1"`) + NowBux/ad-block patcher (stops `BstkSVC.exe` before patching so configs are never overwritten), Kernel IFEO `PerfOptions` permanent High CPU Priority lock, raw VT-x BCD Hypervisor + HVCI toggle, RAII 1.0ms Timer Game Mode Daemon, and 100% Vanguard-safe VALORANT UTF-8 no-BOM INI presets.
  - **Safety Center**: Typed LIFO transaction rollback journals (`%LOCALAPPDATA%\OptimizerByRaghav\journals\`), on-demand Emergency `.bat` undo generator, and Volume Shadow Copy restore point manager.

---

## Build & Run

### Prerequisites
- Windows 10 (21H2+) or Windows 11 (x64)
- Rust toolchain (`rustc` / `cargo`, e.g. via `rustup`) & MSVC Build Tools

### Build Standalone & Setup Release Executables
```powershell
.\build\Build-Release.ps1
```
Produces:
- `dist\OptimizerByRaghav.exe` (Standalone single-file portable executable with UAC `requireAdministrator` + PerMonitorV2 DPI manifest, icon, and PE VersionInfo)
- `dist\OptimizerByRaghav.sha256`
- `dist\OptimizerByRaghav-Setup.exe` (Native Windows Setup Installer wizard embedding `OptimizerByRaghav.exe`)
- `dist\OptimizerByRaghav-Setup.sha256`

### Install or Run
- **Install**: Run `dist\OptimizerByRaghav-Setup.exe` (or `dist\OptimizerByRaghav.exe --install`, or click **Install to Windows** in the Settings tab).
- **Uninstall**: Run `dist\OptimizerByRaghav-Setup.exe --uninstall` (or uninstall from Windows **Installed Apps**).
- **Portable**: Double-click `Launch-OptimizerByRaghav.bat` or `dist\OptimizerByRaghav.exe`.
- Read-only headless hardware/audit diagnostic:
  ```powershell
  cargo run --bin OptimizerByRaghav -- --self-test
  ```
- Run unit tests + live read-only hardware smoke test:
  ```powershell
  cargo test --bin OptimizerByRaghav -- --nocapture
  ```
- Web bootstrapper (`run.ps1`, repo: `https://github.com/raghavatgit/AnxiouslyOptimized`):
  ```powershell
  irm https://raw.githubusercontent.com/raghavatgit/AnxiouslyOptimized/main/run.ps1 | iex
  ```
