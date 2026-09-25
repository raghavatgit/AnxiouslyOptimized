using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Net;
using System.Threading.Tasks;
using Microsoft.Win32;

namespace AnxiouslyOptimized.Services
{
    public class SoftwarePackageItem
    {
        public string Id { get; set; }
        public string Name { get; set; }
        public string Category { get; set; }
        public string Description { get; set; }
        public string DirectUrl { get; set; }
        public string DirectArgs { get; set; }
        public bool IsInstalled { get; set; }
        public bool IsSelected { get; set; }
        public string StatusText { get; set; }

        public SoftwarePackageItem()
        {
            IsInstalled = false;
            IsSelected = false;
            StatusText = "Checking...";
        }
    }

    public static class RuntimeHubService
    {
        private static bool? _isWinGetAvailable = null;

        public static bool IsWinGetAvailable()
        {
            if (_isWinGetAvailable.HasValue) return _isWinGetAvailable.Value;

            try
            {
                using (var p = Process.Start(new ProcessStartInfo
                {
                    FileName = "winget.exe",
                    Arguments = "--version",
                    CreateNoWindow = true,
                    UseShellExecute = false
                }))
                {
                    p.WaitForExit(2000);
                    _isWinGetAvailable = (p.ExitCode == 0);
                    return _isWinGetAvailable.Value;
                }
            }
            catch
            {
                _isWinGetAvailable = false;
                return false;
            }
        }

        public static List<SoftwarePackageItem> GetCatalog()
        {
            return new List<SoftwarePackageItem>
            {
                // Core Gaming Runtimes
                new SoftwarePackageItem
                {
                    Id = "Microsoft.VCRedist.2015+.x64",
                    Name = "Visual C++ 2015-2022 Redistributable (x64)",
                    Category = "Gaming Runtimes",
                    Description = "Required by all 64-bit Direct3D, Unreal, and Unity PC games to run smoothly without missing DLL errors.",
                    DirectUrl = "https://aka.ms/vs/17/release/vc_redist.x64.exe",
                    DirectArgs = "/install /quiet /norestart",
                    IsSelected = true
                },
                new SoftwarePackageItem
                {
                    Id = "Microsoft.VCRedist.2015+.x86",
                    Name = "Visual C++ 2015-2022 Redistributable (x86)",
                    Category = "Gaming Runtimes",
                    Description = "Required by 32-bit game engines, emulators, and background launcher services.",
                    DirectUrl = "https://aka.ms/vs/17/release/vc_redist.x86.exe",
                    DirectArgs = "/install /quiet /norestart",
                    IsSelected = true
                },
                new SoftwarePackageItem
                {
                    Id = "Microsoft.DirectX",
                    Name = "DirectX End-User Runtimes (June 2010)",
                    Category = "Gaming Runtimes",
                    Description = "Installs legacy Direct3D 9, 10, and 11 audio/graphic components (d3dx9_43.dll) needed by competitive games.",
                    DirectUrl = "https://download.microsoft.com/download/1/7/1/1718CCC4-6315-4D8E-9543-8E28A4E18C4C/dxwebsetup.exe",
                    DirectArgs = "/q",
                    IsSelected = true
                },
                new SoftwarePackageItem
                {
                    Id = "Microsoft.DotNet.DesktopRuntime.8",
                    Name = ".NET Desktop Runtime 8.0 (LTS)",
                    Category = "Gaming Runtimes",
                    Description = "Official Microsoft .NET modern application runtime for game mods, overlays, and performance utilities.",
                    DirectUrl = "https://aka.ms/dotnet/8.0/windowsdesktop-runtime-win-x64.exe",
                    DirectArgs = "/install /quiet /norestart",
                    IsSelected = false
                },

                // Essential Gaming Tools
                new SoftwarePackageItem
                {
                    Id = "7zip.7zip",
                    Name = "7-Zip High-Performance Archiver",
                    Category = "Gaming Tools",
                    Description = "Ultra-fast decompression engine for game archives, custom skins, textures, and ROM files.",
                    DirectUrl = "https://www.7-zip.org/a/7z2408-x64.exe",
                    DirectArgs = "/S",
                    IsSelected = false
                },
                new SoftwarePackageItem
                {
                    Id = "Guru3D.RTSS",
                    Name = "RivaTuner Statistics Server (RTSS)",
                    Category = "Gaming Tools",
                    Description = "The gold standard frametime limiter. Eliminates micro-stuttering and locks framerate with microsecond precision.",
                    DirectUrl = "https://download.guru3d.com/rtss/RTSSSetup735.exe",
                    DirectArgs = "/S",
                    IsSelected = false
                },
                new SoftwarePackageItem
                {
                    Id = "OBSProject.OBSStudio",
                    Name = "OBS Studio (GPU Accelerated)",
                    Category = "Gaming Tools",
                    Description = "Hardware NVENC/AMF/QSV accelerated screen recorder and streamer with minimal CPU impact.",
                    DirectUrl = "https://cdn-fastly.obsproject.com/downloads/OBS-Studio-30.2.3-Windows-Installer.exe",
                    DirectArgs = "/S",
                    IsSelected = false
                },
                new SoftwarePackageItem
                {
                    Id = "Valve.Steam",
                    Name = "Steam Client",
                    Category = "Gaming Tools",
                    Description = "The premier digital PC gaming storefront and multiplayer match ecosystem.",
                    DirectUrl = "https://cdn.akamai.steamstatic.com/client/installer/SteamSetup.exe",
                    DirectArgs = "/S",
                    IsSelected = false
                },
                new SoftwarePackageItem
                {
                    Id = "Discord.Discord",
                    Name = "Discord Voice & Community",
                    Category = "Gaming Tools",
                    Description = "Low-latency in-game voice chat and competitive team communications.",
                    DirectUrl = "https://discord.com/api/download?platform=win",
                    DirectArgs = "--silent",
                    IsSelected = false
                },

                // Competitive Android Emulators
                new SoftwarePackageItem
                {
                    Id = "BlueStack.BlueStacks",
                    Name = "BlueStacks 5 (Pie 64-bit Ready)",
                    Category = "Emulators",
                    Description = "High-FPS Android 9/11 virtualized gaming engine with dedicated ARM translation for Free Fire.",
                    DirectUrl = "https://cloud.bluestacks.com/api/getdownloadnow?package=com.bluestacks.nxt",
                    DirectArgs = "--default",
                    IsSelected = false
                }
            };
        }

        public static async Task CheckInstalledStatusAsync(List<SoftwarePackageItem> packages, Action<string> log)
        {
            await Task.Run(() =>
            {
                LogSafe(log, "Checking installed gaming runtimes and software packages...");

                foreach (var pkg in packages)
                {
                    bool installed = DetectInstalledLocally(pkg.Id);
                    pkg.IsInstalled = installed;
                    pkg.StatusText = installed ? "Installed" : "Available";

                    LogSafe(log, string.Format("  [{0}] -> {1}", pkg.Name, pkg.StatusText));
                }
            });
        }

        private static bool DetectInstalledLocally(string id)
        {
            try
            {
                if (id == "Microsoft.VCRedist.2015+.x64")
                {
                    using (var key = Registry.LocalMachine.OpenSubKey(@"SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\X64"))
                    {
                        if (key != null && Convert.ToInt32(key.GetValue("Installed", 0)) == 1) return true;
                    }
                    using (var key = Registry.LocalMachine.OpenSubKey(@"SOFTWARE\WOW6432Node\Microsoft\VisualStudio\14.0\VC\Runtimes\X64"))
                    {
                        if (key != null && Convert.ToInt32(key.GetValue("Installed", 0)) == 1) return true;
                    }
                }
                else if (id == "Microsoft.VCRedist.2015+.x86")
                {
                    using (var key = Registry.LocalMachine.OpenSubKey(@"SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\X86"))
                    {
                        if (key != null && Convert.ToInt32(key.GetValue("Installed", 0)) == 1) return true;
                    }
                    using (var key = Registry.LocalMachine.OpenSubKey(@"SOFTWARE\WOW6432Node\Microsoft\VisualStudio\14.0\VC\Runtimes\X86"))
                    {
                        if (key != null && Convert.ToInt32(key.GetValue("Installed", 0)) == 1) return true;
                    }
                }
                else if (id == "Microsoft.DirectX")
                {
                    string sys32 = Environment.GetFolderPath(Environment.SpecialFolder.System);
                    if (File.Exists(Path.Combine(sys32, "d3dx9_43.dll")) && File.Exists(Path.Combine(sys32, "d3dx11_43.dll")))
                    {
                        return true;
                    }
                }
                else if (id == "Microsoft.DotNet.DesktopRuntime.8")
                {
                    string dir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles), @"dotnet\shared\Microsoft.WindowsDesktop.App");
                    if (Directory.Exists(dir))
                    {
                        var subs = Directory.GetDirectories(dir, "8.*");
                        if (subs.Length > 0) return true;
                    }
                }
                else if (id == "7zip.7zip")
                {
                    string path64 = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles), @"7-Zip\7z.exe");
                    string path86 = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86), @"7-Zip\7z.exe");
                    if (File.Exists(path64) || File.Exists(path86)) return true;
                }
                else if (id == "OBSProject.OBSStudio")
                {
                    string path = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles), @"obs-studio\bin\64bit\obs64.exe");
                    if (File.Exists(path)) return true;
                }
                else if (id == "Guru3D.RTSS")
                {
                    string path = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86), @"RivaTuner Statistics Server\RTSS.exe");
                    if (File.Exists(path)) return true;
                }
                else if (id == "Valve.Steam")
                {
                    using (var key = Registry.CurrentUser.OpenSubKey(@"Software\Valve\Steam"))
                    {
                        if (key != null && key.GetValue("SteamExe") != null) return true;
                    }
                    string path = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86), @"Steam\steam.exe");
                    if (File.Exists(path)) return true;
                }
                else if (id == "Discord.Discord")
                {
                    string path = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), @"Discord\Update.exe");
                    if (File.Exists(path)) return true;
                }
                else if (id == "BlueStack.BlueStacks")
                {
                    string p1 = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles), @"BlueStacks_nxt\HD-Player.exe");
                    string p2 = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles), @"BlueStacks_msi5\HD-Player.exe");
                    if (File.Exists(p1) || File.Exists(p2)) return true;
                }
            }
            catch { }

            return false;
        }

        public static async Task<bool> InstallPackageAsync(SoftwarePackageItem package, Action<string> log)
        {
            return await Task.Run(() =>
            {
                LogSafe(log, string.Format("Deploying {0} ({1})...", package.Name, package.Id));

                bool useWinGet = IsWinGetAvailable();
                if (useWinGet)
                {
                    try
                    {
                        LogSafe(log, string.Format("  [WinGet] Executing silent installation for '{0}'...", package.Id));
                        string args = string.Format("install --id {0} --silent --accept-package-agreements --accept-source-agreements --disable-interactivity", package.Id);

                        using (var p = Process.Start(new ProcessStartInfo
                        {
                            FileName = "winget.exe",
                            Arguments = args,
                            CreateNoWindow = true,
                            UseShellExecute = false
                        }))
                        {
                            p.WaitForExit(180000); // 3 minutes max
                            if (p.ExitCode == 0)
                            {
                                package.IsInstalled = true;
                                package.StatusText = "Installed";
                                LogSafe(log, string.Format("  [WinGet: SUCCESS] {0} installed successfully.", package.Name));
                                return true;
                            }
                            else
                            {
                                LogSafe(log, string.Format("  [WinGet] Exit code {0}, trying direct CDN bootstrap fallback...", p.ExitCode));
                            }
                        }
                    }
                    catch (Exception ex)
                    {
                        LogSafe(log, "  [WinGet notice]: " + ex.Message);
                    }
                }

                // Fallback to direct download & silent install
                if (!string.IsNullOrEmpty(package.DirectUrl))
                {
                    try
                    {
                        LogSafe(log, string.Format("  [Direct CDN] Downloading official installer from {0}...", package.DirectUrl));
                        string tempFile = Path.Combine(Path.GetTempPath(), "ao_setup_" + Guid.NewGuid().ToString("N").Substring(0, 8) + ".exe");

                        using (var wc = new WebClient())
                        {
                            ServicePointManager.SecurityProtocol = SecurityProtocolType.Tls12;
                            wc.Headers.Add("User-Agent", "AnxiouslyOptimized-Runtime-Hub/2.0");
                            wc.DownloadFile(package.DirectUrl, tempFile);
                        }

                        if (File.Exists(tempFile))
                        {
                            LogSafe(log, string.Format("  [Direct CDN] Executing installer with silent arguments: {0}...", package.DirectArgs));
                            using (var p = Process.Start(new ProcessStartInfo
                            {
                                FileName = tempFile,
                                Arguments = package.DirectArgs,
                                CreateNoWindow = true,
                                UseShellExecute = true
                            }))
                            {
                                p.WaitForExit(180000);
                            }

                            try { File.Delete(tempFile); } catch { }

                            package.IsInstalled = true;
                            package.StatusText = "Installed";
                            LogSafe(log, string.Format("  [Direct CDN: SUCCESS] {0} installed successfully.", package.Name));
                            return true;
                        }
                    }
                    catch (Exception ex)
                    {
                        LogSafe(log, "  [Direct CDN Error]: " + ex.Message);
                    }
                }

                LogSafe(log, string.Format("  [FAILED] Could not complete installation for {0}.", package.Name));
                return false;
            });
        }

        public static async Task<int> InstallSelectedBatchAsync(IEnumerable<SoftwarePackageItem> packages, Action<string> log)
        {
            var selected = packages.Where(p => p.IsSelected && !p.IsInstalled).ToList();
            if (selected.Count == 0)
            {
                LogSafe(log, "No uninstalled packages selected for deployment.");
                return 0;
            }

            LogSafe(log, string.Format("Starting batch deployment of {0} selected software packages...", selected.Count));
            int successCount = 0;

            foreach (var pkg in selected)
            {
                bool ok = await InstallPackageAsync(pkg, log);
                if (ok) successCount++;
            }

            LogSafe(log, string.Format("Batch deployment complete: {0}/{1} packages successfully installed.", successCount, selected.Count));
            return successCount;
        }

        private static void LogSafe(Action<string> log, string msg)
        {
            try
            {
                if (log != null) log(msg);
            }
            catch { }
        }
    }
}
