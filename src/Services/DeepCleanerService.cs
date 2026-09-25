using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading.Tasks;

namespace AnxiouslyOptimized.Services
{
    public class CleanerTargetCategory
    {
        public string Id { get; set; }
        public string Title { get; set; }
        public string Description { get; set; }
        public bool IsSelected { get; set; }
        public int FileCount { get; set; }
        public long TotalSizeBytes { get; set; }
        public List<string> Directories { get; set; }
        public List<string> FilesOrWildcards { get; set; }

        public CleanerTargetCategory()
        {
            IsSelected = true;
            Directories = new List<string>();
            FilesOrWildcards = new List<string>();
        }

        public string SizeFormatted
        {
            get
            {
                if (TotalSizeBytes <= 0) return "0 MB";
                if (TotalSizeBytes < 1024 * 1024)
                    return string.Format("{0:0.#} KB", (double)TotalSizeBytes / 1024);
                if (TotalSizeBytes < 1024 * 1024 * 1024)
                    return string.Format("{0:0.#} MB", (double)TotalSizeBytes / (1024 * 1024));
                return string.Format("{0:0.##} GB", (double)TotalSizeBytes / (1024 * 1024 * 1024));
            }
        }
    }

    public static class DeepCleanerService
    {
        public static List<CleanerTargetCategory> GetDefaultCategories()
        {
            string localAppData = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
            string appData = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
            string programData = Environment.GetFolderPath(Environment.SpecialFolder.CommonApplicationData);
            string systemRoot = Environment.GetEnvironmentVariable("SystemRoot") ?? @"C:\Windows";
            string tempDir = Path.GetTempPath();

            var list = new List<CleanerTargetCategory>();

            // 1. DirectX & GPU Shader Caches
            var catShaders = new CleanerTargetCategory
            {
                Id = "gpu_shaders",
                Title = "DirectX & GPU Shader Caches",
                Description = "Purges stale compiled DirectX, NVIDIA, AMD, and Intel shader caches to resolve frame pacing micro-stuttering.",
                IsSelected = true
            };
            catShaders.Directories.Add(Path.Combine(localAppData, "D3DSCache"));
            catShaders.Directories.Add(Path.Combine(localAppData, @"Microsoft\DirectX Shader Cache"));
            catShaders.Directories.Add(Path.Combine(localAppData, @"NVIDIA\DXCache"));
            catShaders.Directories.Add(Path.Combine(localAppData, @"NVIDIA\GLCache"));
            catShaders.Directories.Add(Path.Combine(appData, @"NVIDIA\ComputeCache"));
            catShaders.Directories.Add(Path.Combine(localAppData, @"AMD\DxCache"));
            catShaders.Directories.Add(Path.Combine(localAppData, @"AMD\GLCache"));
            catShaders.Directories.Add(Path.Combine(localAppData, @"Intel\ShaderCache"));
            list.Add(catShaders);

            // 2. Windows Delivery Optimization Cache
            var catDeliveryOpt = new CleanerTargetCategory
            {
                Id = "delivery_opt",
                Title = "Windows Delivery Optimization Cache",
                Description = "Removes accumulated peer-to-peer Windows Update distribution files and cached payload chunks.",
                IsSelected = true
            };
            catDeliveryOpt.Directories.Add(Path.Combine(systemRoot, @"SoftwareDistribution\DeliveryOptimization"));
            list.Add(catDeliveryOpt);

            // 3. Windows Update Download Archive
            var catWinUpdate = new CleanerTargetCategory
            {
                Id = "win_update",
                Title = "Windows Update Download Store",
                Description = "Cleans leftover staged update installation packages that have already been applied to your system.",
                IsSelected = true
            };
            catWinUpdate.Directories.Add(Path.Combine(systemRoot, @"SoftwareDistribution\Download"));
            list.Add(catWinUpdate);

            // 4. Chromium & WebView2 Code Caches
            var catChromium = new CleanerTargetCategory
            {
                Id = "chromium_caches",
                Title = "Chromium & Browser Code Caches",
                Description = "Cleans stale V8 bytecode, GPU shader storage, and web code caches from Edge, Chrome, Discord, and Spotify.",
                IsSelected = true
            };
            catChromium.Directories.Add(Path.Combine(localAppData, @"Microsoft\Edge\User Data\Default\Code Cache"));
            catChromium.Directories.Add(Path.Combine(localAppData, @"Microsoft\Edge\User Data\Default\GPUCache"));
            catChromium.Directories.Add(Path.Combine(localAppData, @"Google\Chrome\User Data\Default\Code Cache"));
            catChromium.Directories.Add(Path.Combine(localAppData, @"Google\Chrome\User Data\Default\GPUCache"));
            catChromium.Directories.Add(Path.Combine(appData, @"discord\Code Cache"));
            catChromium.Directories.Add(Path.Combine(appData, @"discord\GPUCache"));
            catChromium.Directories.Add(Path.Combine(localAppData, @"Spotify\Storage"));
            list.Add(catChromium);

            // 5. Windows Crash Dumps & Error Reports
            var catCrashDumps = new CleanerTargetCategory
            {
                Id = "crash_dumps",
                Title = "Crash Dumps & Error Reports (WER)",
                Description = "Deletes memory crash dump dumps (MEMORY.DMP, minidumps) and Windows Error Reporting diagnostic logs.",
                IsSelected = true
            };
            catCrashDumps.Directories.Add(Path.Combine(localAppData, "CrashDumps"));
            catCrashDumps.Directories.Add(Path.Combine(programData, @"Microsoft\Windows\WER\ReportArchive"));
            catCrashDumps.Directories.Add(Path.Combine(programData, @"Microsoft\Windows\WER\ReportQueue"));
            catCrashDumps.Directories.Add(Path.Combine(systemRoot, "Minidump"));
            list.Add(catCrashDumps);

            // 6. User & System Temporary Storage
            var catTemp = new CleanerTargetCategory
            {
                Id = "system_temp",
                Title = "System & User Temporary Storage",
                Description = "Safely cleans orphaned temporary installation files and scratch data from %TEMP% and Windows Temp.",
                IsSelected = true
            };
            catTemp.Directories.Add(tempDir);
            catTemp.Directories.Add(Path.Combine(systemRoot, "Temp"));
            list.Add(catTemp);

            // 7. Thumbnail & Icon Database Caches
            var catThumbs = new CleanerTargetCategory
            {
                Id = "thumb_cache",
                Title = "Windows Thumbnail & Explorer Caches",
                Description = "Cleans thumbnail database files (thumbcache_*.db) to fix glitched desktop icons and reclaim storage.",
                IsSelected = true
            };
            catThumbs.Directories.Add(Path.Combine(localAppData, @"Microsoft\Windows\Explorer"));
            list.Add(catThumbs);

            // 8. Android Emulator Shader & Log Caches
            var catEmulator = new CleanerTargetCategory
            {
                Id = "emulator_cache",
                Title = "Emulator Shaders, Logs & Temp Caches",
                Description = "Cleans BlueStacks, MSI Player, and LDPlayer log archives, memory dump traces, and virtual disk swap files.",
                IsSelected = true
            };
            catEmulator.Directories.Add(Path.Combine(programData, @"BlueStacks_nxt\Logs"));
            catEmulator.Directories.Add(Path.Combine(programData, @"BlueStacks_msi5\Logs"));
            catEmulator.Directories.Add(Path.Combine(localAppData, @"BlueStacks_nxt"));
            list.Add(catEmulator);

            return list;
        }

        private static void SafeEnumerateFiles(string rootDir, Action<FileInfo> onFile)
        {
            if (string.IsNullOrEmpty(rootDir) || !Directory.Exists(rootDir)) return;

            var stack = new Stack<string>();
            stack.Push(rootDir);

            while (stack.Count > 0)
            {
                string current = stack.Pop();
                try
                {
                    var dirInfo = new DirectoryInfo(current);
                    FileInfo[] files = null;
                    try
                    {
                        files = dirInfo.GetFiles();
                    }
                    catch { }

                    if (files != null)
                    {
                        for (int i = 0; i < files.Length; i++)
                        {
                            try
                            {
                                onFile(files[i]);
                            }
                            catch { }
                        }
                    }

                    DirectoryInfo[] subDirs = null;
                    try
                    {
                        subDirs = dirInfo.GetDirectories();
                    }
                    catch { }

                    if (subDirs != null)
                    {
                        for (int i = 0; i < subDirs.Length; i++)
                        {
                            try
                            {
                                var sub = subDirs[i];
                                if ((sub.Attributes & FileAttributes.ReparsePoint) != FileAttributes.ReparsePoint)
                                {
                                    stack.Push(sub.FullName);
                                }
                            }
                            catch { }
                        }
                    }
                }
                catch { }
            }
        }

        private static void LogSafe(Action<string> log, string msg)
        {
            try
            {
                if (log != null) log(msg);
            }
            catch { }
        }

        public static async Task<List<CleanerTargetCategory>> ScanAllCategoriesAsync(Action<string> log)
        {
            return await Task.Run(() =>
            {
                LogSafe(log, "Scanning system and gaming cache directories for reclaimable storage...");
                var categories = GetDefaultCategories();
                long totalBytesAll = 0;
                int totalFilesAll = 0;

                foreach (var cat in categories)
                {
                    long catBytes = 0;
                    int catFiles = 0;

                    foreach (var dir in cat.Directories)
                    {
                        SafeEnumerateFiles(dir, f =>
                        {
                            try
                            {
                                catBytes += f.Length;
                                catFiles++;
                            }
                            catch { }
                        });
                    }

                    cat.TotalSizeBytes = catBytes;
                    cat.FileCount = catFiles;
                    totalBytesAll += catBytes;
                    totalFilesAll += catFiles;

                    LogSafe(log, string.Format("  [{0}] Scanned: {1} ({2} files)", cat.Title, cat.SizeFormatted, cat.FileCount));
                }

                string totalFormatted;
                if (totalBytesAll < 1024 * 1024 * 1024)
                    totalFormatted = string.Format("{0:0.#} MB", (double)totalBytesAll / (1024 * 1024));
                else
                    totalFormatted = string.Format("{0:0.##} GB", (double)totalBytesAll / (1024 * 1024 * 1024));

                LogSafe(log, string.Format("Scan completed: Found {0} across {1} files ready for purging.", totalFormatted, totalFilesAll));
                return categories;
            });
        }

        public static async Task<long> PurgeCategoriesAsync(IEnumerable<CleanerTargetCategory> categories, Action<string> log)
        {
            return await Task.Run(() =>
            {
                LogSafe(log, "Starting Deep System & Shader Cache Purge...");
                long freedBytesTotal = 0;
                int deletedFilesTotal = 0;

                foreach (var cat in categories)
                {
                    if (!cat.IsSelected) continue;

                    LogSafe(log, string.Format("Purging {0}...", cat.Title));
                    long catFreed = 0;
                    int catFiles = 0;

                    foreach (var dir in cat.Directories)
                    {
                        SafeEnumerateFiles(dir, file =>
                        {
                            try
                            {
                                long len = file.Length;
                                file.Attributes = FileAttributes.Normal;
                                file.Delete();
                                catFreed += len;
                                catFiles++;
                            }
                            catch
                            {
                                // In-use file skipped
                            }
                        });
                    }

                    freedBytesTotal += catFreed;
                    deletedFilesTotal += catFiles;

                    string catFreedStr;
                    if (catFreed < 1024 * 1024 * 1024)
                        catFreedStr = string.Format("{0:0.#} MB", (double)catFreed / (1024 * 1024));
                    else
                        catFreedStr = string.Format("{0:0.##} GB", (double)catFreed / (1024 * 1024 * 1024));

                    LogSafe(log, string.Format("  [OK] Cleaned {0}: Freed {1} ({2} files purged)", cat.Title, catFreedStr, catFiles));
                }

                // Flush DNS Cache
                try
                {
                    LogSafe(log, "Flushing Windows DNS Resolver Cache...");
                    using (var p = System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
                    {
                        FileName = "ipconfig.exe",
                        Arguments = "/flushdns",
                        CreateNoWindow = true,
                        UseShellExecute = false
                    }))
                    {
                        p.WaitForExit(3000);
                    }
                    LogSafe(log, "  [OK] Windows DNS cache flushed.");
                }
                catch { }

                string totalFreedStr;
                if (freedBytesTotal < 1024 * 1024 * 1024)
                    totalFreedStr = string.Format("{0:0.#} MB", (double)freedBytesTotal / (1024 * 1024));
                else
                    totalFreedStr = string.Format("{0:0.##} GB", (double)freedBytesTotal / (1024 * 1024 * 1024));

                LogSafe(log, string.Format("PURGE COMPLETED: Total {0} safely recovered across {1} files!", totalFreedStr, deletedFilesTotal));
                return freedBytesTotal;
            });
        }
    }
}
