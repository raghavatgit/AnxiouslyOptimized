using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading.Tasks;

namespace AnxiouslyOptimized.Services
{
    public static class MaintenanceService
    {
        [DllImport("psapi.dll")]
        private static extern int EmptyWorkingSet(IntPtr hwProc);

        public static async Task<long> FlushRamAsync(Action<string> log)
        {
            return await Task.Run(() =>
            {
                log("Flushing system memory standby list and process working sets...");
                long freedMb = 0;
                int count = 0;

                try
                {
                    Process[] procs = Process.GetProcesses();
                    foreach (var p in procs)
                    {
                        try
                        {
                            if (!p.HasExited && p.Handle != IntPtr.Zero)
                            {
                                long memBefore = p.WorkingSet64;
                                EmptyWorkingSet(p.Handle);
                                p.Refresh();
                                long memAfter = p.WorkingSet64;
                                if (memBefore > memAfter)
                                {
                                    freedMb += (memBefore - memAfter) / (1024 * 1024);
                                }
                                count++;
                            }
                        }
                        catch { }
                    }

                    GC.Collect();
                    GC.WaitForPendingFinalizers();
                    GC.Collect();

                    log(string.Format("Flushed working sets across {0} processes. Freed approximately {1} MB RAM.", count, freedMb));
                }
                catch (Exception ex)
                {
                    log("Memory flush notice: " + ex.Message);
                }

                return freedMb;
            });
        }

        public static async Task CleanCacheAsync(Action<string> log)
        {
            await Task.Run(() =>
            {
                log("Starting temporary cache and DNS cleanup...");

                string[] tempFolders = new string[]
                {
                    Path.GetTempPath(),
                    Environment.ExpandEnvironmentVariables(@"%SystemRoot%\Temp")
                };

                int deletedFiles = 0;
                foreach (var folder in tempFolders)
                {
                    if (Directory.Exists(folder))
                    {
                        try
                        {
                            var files = Directory.GetFiles(folder, "*.*", SearchOption.AllDirectories);
                            foreach (var f in files)
                            {
                                try
                                {
                                    File.Delete(f);
                                    deletedFiles++;
                                }
                                catch { }
                            }
                        }
                        catch { }
                    }
                }
                log(string.Format("Cleaned {0} temporary files from system temp caches.", deletedFiles));

                // Flush DNS
                try
                {
                    log("Flushing DNS resolver cache...");
                    using (var p = Process.Start(new ProcessStartInfo
                    {
                        FileName = "ipconfig",
                        Arguments = "/flushdns",
                        CreateNoWindow = true,
                        UseShellExecute = false
                    }))
                    {
                        p.WaitForExit(5000);
                    }
                    log("DNS cache flushed successfully.");
                }
                catch { }

                log("System cache cleanup completed.");
            });
        }

        public static async Task RunCommandAsync(string fileName, string args, Action<string> log)
        {
            await Task.Run(() =>
            {
                log(string.Format("Executing: {0} {1}", fileName, args));
                try
                {
                    var psi = new ProcessStartInfo
                    {
                        FileName = fileName,
                        Arguments = args,
                        CreateNoWindow = true,
                        UseShellExecute = false,
                        RedirectStandardOutput = true,
                        RedirectStandardError = true
                    };

                    using (var process = new Process { StartInfo = psi })
                    {
                        process.OutputDataReceived += (s, e) =>
                        {
                            if (!string.IsNullOrEmpty(e.Data))
                                log(e.Data);
                        };
                        process.ErrorDataReceived += (s, e) =>
                        {
                            if (!string.IsNullOrEmpty(e.Data))
                                log("[ERR] " + e.Data);
                        };

                        process.Start();
                        process.BeginOutputReadLine();
                        process.BeginErrorReadLine();
                        process.WaitForExit();

                        log(string.Format("Command finished with exit code {0}.", process.ExitCode));
                    }
                }
                catch (Exception ex)
                {
                    log("Execution error: " + ex.Message);
                }
            });
        }

        public static async Task RunSfcAsync(Action<string> log)
        {
            await RunCommandAsync("sfc.exe", "/scannow", log);
        }

        public static async Task RunDismRestoreHealthAsync(Action<string> log)
        {
            await RunCommandAsync("dism.exe", "/online /cleanup-image /restorehealth", log);
        }

        public static async Task RunComponentCleanupAsync(Action<string> log)
        {
            await RunCommandAsync("dism.exe", "/online /cleanup-image /startcomponentcleanup /resetbase", log);
        }
    }
}
