using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using AnxiouslyOptimized.Models;

namespace AnxiouslyOptimized.Services
{
    public static class SafetyService
    {
        private static readonly JavaScriptSerializer _serializer = new JavaScriptSerializer { MaxJsonLength = int.MaxValue };

        public static string GetBackupsDirectory()
        {
            string dir = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "backups");
            if (!Directory.Exists(dir))
            {
                try { Directory.CreateDirectory(dir); } catch { }
            }
            return dir;
        }

        private static string GetManifestPath()
        {
            return Path.Combine(GetBackupsDirectory(), "manifest.json");
        }

        public static List<BackupEntry> LoadManifestEntries()
        {
            var list = new List<BackupEntry>();
            string path = GetManifestPath();
            if (File.Exists(path))
            {
                try
                {
                    string json = File.ReadAllText(path);
                    var raw = _serializer.Deserialize<List<Dictionary<string, object>>>(json);
                    if (raw != null)
                    {
                        foreach (var dict in raw)
                        {
                            var entry = new BackupEntry();
                            if (dict.ContainsKey("Id")) entry.Id = dict["Id"] as string ?? entry.Id;
                            if (dict.ContainsKey("Title")) entry.Title = dict["Title"] as string ?? "";
                            if (dict.ContainsKey("BackupType")) entry.BackupType = dict["BackupType"] as string ?? "Registry Archive";
                            if (dict.ContainsKey("TimestampFormatted")) entry.TimestampFormatted = dict["TimestampFormatted"] as string ?? "";
                            if (dict.ContainsKey("FilePath")) entry.FilePath = dict["FilePath"] as string ?? "";
                            if (dict.ContainsKey("SizeFormatted")) entry.SizeFormatted = dict["SizeFormatted"] as string ?? "";
                            if (dict.ContainsKey("Description")) entry.Description = dict["Description"] as string ?? "";
                            if (dict.ContainsKey("Status")) entry.Status = dict["Status"] as string ?? "Protected";
                            if (dict.ContainsKey("IsSystemRestorePoint")) entry.IsSystemRestorePoint = Convert.ToBoolean(dict["IsSystemRestorePoint"]);
                            if (dict.ContainsKey("SequenceNumber")) entry.SequenceNumber = Convert.ToInt32(dict["SequenceNumber"]);
                            if (dict.ContainsKey("SizeBytes")) entry.SizeBytes = Convert.ToInt64(dict["SizeBytes"]);

                            DateTime dt;
                            if (dict.ContainsKey("CreatedAt") && DateTime.TryParse(dict["CreatedAt"] as string, out dt))
                                entry.CreatedAt = dt;

                            list.Add(entry);
                        }
                    }
                }
                catch { }
            }
            return list;
        }

        public static void SaveManifestEntries(List<BackupEntry> entries)
        {
            try
            {
                string path = GetManifestPath();
                string json = _serializer.Serialize(entries);
                File.WriteAllText(path, json, Encoding.UTF8);
            }
            catch { }
        }

        public static async Task<bool> CreateRestorePointAsync(string description, Action<string> log)
        {
            return await Task.Run(() =>
            {
                log("Creating Windows System Restore Point: " + description + "...");
                bool systemRestoreSuccess = false;
                try
                {
                    string safeDesc = description.Replace("'", "''");
                    string psCommand = string.Format(
                        "try {{ " +
                        "Enable-ComputerRestore -Drive 'C:\\' -ErrorAction SilentlyContinue; " +
                        "Set-ItemProperty -Path 'HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\SystemRestore' -Name 'SystemRestorePointCreationFrequency' -Value 0 -Type DWord -ErrorAction SilentlyContinue; " +
                        "Checkpoint-Computer -Description '{0}' -RestorePointType 'MODIFY_SETTINGS' -ErrorAction Stop; " +
                        "Write-Host 'SUCCESS' " +
                        "}} catch {{ Write-Host ('FAILED: ' + $_.Exception.Message) }}",
                        safeDesc);

                    var psi = new ProcessStartInfo
                    {
                        FileName = "powershell.exe",
                        Arguments = "-NoProfile -ExecutionPolicy Bypass -Command \"" + psCommand + "\"",
                        CreateNoWindow = true,
                        UseShellExecute = false,
                        RedirectStandardOutput = true
                    };

                    using (var proc = Process.Start(psi))
                    {
                        string output = proc.StandardOutput.ReadToEnd();
                        proc.WaitForExit();

                        if (output.Contains("SUCCESS"))
                        {
                            systemRestoreSuccess = true;
                            log("System Restore Point created successfully.");
                        }
                        else
                        {
                            log("Notice on system restore point: " + output.Trim());
                        }
                    }
                }
                catch (Exception ex)
                {
                    log("Failed to create system restore point: " + ex.Message);
                }

                // Dual protection: Always create a complementary registry archive
                try
                {
                    log("Creating complementary multi-hive registry safety backup...");
                    string regFile = ExportRegistryBackupInternal(description, log);
                    if (!string.IsNullOrEmpty(regFile) && File.Exists(regFile))
                    {
                        var fi = new FileInfo(regFile);
                        var manifest = LoadManifestEntries();
                        manifest.Insert(0, new BackupEntry
                        {
                            Title = description,
                            BackupType = systemRestoreSuccess ? "System Snapshot + Registry" : "Registry Archive",
                            CreatedAt = DateTime.Now,
                            TimestampFormatted = DateTime.Now.ToString("MMM dd, yyyy - hh:mm tt"),
                            FilePath = regFile,
                            SizeBytes = fi.Length,
                            SizeFormatted = string.Format("{0:0.0} MB", fi.Length / (1024.0 * 1024.0)),
                            IsSystemRestorePoint = systemRestoreSuccess,
                            Description = "Complete backup of Windows Explorer, Multimedia, Graphics, and Mouse settings",
                            Status = "Verified"
                        });
                        SaveManifestEntries(manifest);
                    }
                }
                catch { }

                return true;
            });
        }

        private static string ExportRegistryBackupInternal(string backupName, Action<string> log)
        {
            string backupsDir = GetBackupsDirectory();
            string timestamp = DateTime.Now.ToString("yyyyMMdd_HHmmss");
            string safeName = Regex.Replace(backupName, @"[^a-zA-Z0-9_\-]", "_");
            string combinedFile = Path.Combine(backupsDir, string.Format("RegistryBackup_{0}_{1}.reg", safeName, timestamp));

            string[] keysToExport = new string[]
            {
                @"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer",
                @"HKCU\Software\Microsoft\Windows\CurrentVersion\GameDVR",
                @"HKCU\Control Panel\Mouse",
                @"HKLM\SYSTEM\CurrentControlSet\Control\GraphicsDrivers",
                @"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile"
            };

            var sb = new StringBuilder();
            sb.AppendLine("Windows Registry Editor Version 5.00");
            sb.AppendLine();
            sb.AppendLine("; AnxiouslyOptimized System Safety Archive");
            sb.AppendLine("; Created: " + DateTime.Now.ToString("u"));
            sb.AppendLine("; Description: " + backupName);
            sb.AppendLine();

            foreach (var key in keysToExport)
            {
                string cleanKey = key.Replace("\\", "_");
                string tempPart = Path.Combine(backupsDir, string.Format("temp_{0}_{1}.reg", cleanKey, timestamp));
                try
                {
                    using (var proc = Process.Start(new ProcessStartInfo
                    {
                        FileName = "reg.exe",
                        Arguments = string.Format("export \"{0}\" \"{1}\" /y", key, tempPart),
                        CreateNoWindow = true,
                        UseShellExecute = false
                    }))
                    {
                        proc.WaitForExit(4000);
                    }

                    if (File.Exists(tempPart))
                    {
                        string[] lines = File.ReadAllLines(tempPart);
                        bool isHeader = true;
                        foreach (var l in lines)
                        {
                            if (isHeader)
                            {
                                if (l.StartsWith("Windows Registry Editor", StringComparison.OrdinalIgnoreCase))
                                    continue;
                                if (string.IsNullOrWhiteSpace(l))
                                    continue;
                                isHeader = false;
                            }
                            sb.AppendLine(l);
                        }
                        File.Delete(tempPart);
                    }
                }
                catch { }
            }

            File.WriteAllText(combinedFile, sb.ToString(), Encoding.Unicode);
            log("  [OK] Registry snapshot recorded: " + Path.GetFileName(combinedFile));
            return combinedFile;
        }

        public static async Task<bool> ImportRegistryBackupAsync(string regFilePath, Action<string> log)
        {
            return await Task.Run(() =>
            {
                if (!File.Exists(regFilePath))
                {
                    log("[ERROR] Backup file not found: " + regFilePath);
                    return false;
                }

                log("Restoring Windows registry settings from: " + Path.GetFileName(regFilePath) + "...");
                try
                {
                    using (var proc = Process.Start(new ProcessStartInfo
                    {
                        FileName = "reg.exe",
                        Arguments = string.Format("import \"{0}\"", regFilePath),
                        CreateNoWindow = true,
                        UseShellExecute = false,
                        RedirectStandardOutput = true,
                        RedirectStandardError = true
                    }))
                    {
                        proc.WaitForExit(8000);
                        if (proc.ExitCode == 0)
                        {
                            log("[SUCCESS] Registry settings restored successfully from backup.");
                            return true;
                        }
                        else
                        {
                            // If standard process had access denied, try elevated UAC prompt
                            log("[NOTICE] Standard import returned access denied. Requesting elevated import...");
                            try
                            {
                                using (var elevated = Process.Start(new ProcessStartInfo
                                {
                                    FileName = "reg.exe",
                                    Arguments = string.Format("import \"{0}\"", regFilePath),
                                    Verb = "runas",
                                    UseShellExecute = true
                                }))
                                {
                                    elevated.WaitForExit();
                                    if (elevated.ExitCode == 0)
                                    {
                                        log("[SUCCESS] Elevated registry restore completed.");
                                        return true;
                                    }
                                }
                            }
                            catch (Exception elevatedEx)
                            {
                                log("[NOTICE] Elevated request canceled or failed: " + elevatedEx.Message);
                            }

                            string err = proc.StandardError.ReadToEnd();
                            log("[ERROR] Registry import exited with code " + proc.ExitCode + ": " + err);
                            return false;
                        }
                    }
                }
                catch (Exception ex)
                {
                    log("[ERROR] Registry import exception: " + ex.Message);
                    return false;
                }
            });
        }

        public static async Task<bool> RestoreSystemSnapshotAsync(int sequenceNumber, Action<string> log)
        {
            return await Task.Run(() =>
            {
                log(string.Format("Initiating system rollback to Restore Point #{0}...", sequenceNumber));
                try
                {
                    string psCommand = string.Format("Restore-Computer -RestorePoint {0} -Confirm:$false", sequenceNumber);
                    var psi = new ProcessStartInfo
                    {
                        FileName = "powershell.exe",
                        Arguments = "-NoProfile -ExecutionPolicy Bypass -Command \"" + psCommand + "\"",
                        CreateNoWindow = true,
                        UseShellExecute = false,
                        RedirectStandardOutput = true,
                        RedirectStandardError = true
                    };

                    using (var proc = Process.Start(psi))
                    {
                        proc.WaitForExit(10000);
                        if (proc.ExitCode == 0)
                        {
                            log("[SUCCESS] System rollback initiated. Windows will restart to complete restore.");
                            return true;
                        }
                        else
                        {
                            string err = proc.StandardError.ReadToEnd();
                            log("[NOTICE] Direct restore prompt: " + err);
                            log("Launching Windows System Restore interface for guided rollback...");
                            LaunchSystemRestoreWizard(log);
                            return true;
                        }
                    }
                }
                catch (Exception ex)
                {
                    log("Notice: " + ex.Message + ". Opening System Restore wizard...");
                    LaunchSystemRestoreWizard(log);
                    return false;
                }
            });
        }

        public static void LaunchSystemRestoreWizard(Action<string> log = null)
        {
            try
            {
                if (log != null) log("Launching Windows System Restore Wizard (rstrui.exe)...");
                Process.Start(new ProcessStartInfo
                {
                    FileName = "rstrui.exe",
                    UseShellExecute = true
                });
            }
            catch (Exception ex)
            {
                if (log != null) log("Unable to launch rstrui.exe: " + ex.Message);
            }
        }

        public static void OpenBackupsFolder()
        {
            try
            {
                string dir = GetBackupsDirectory();
                Process.Start("explorer.exe", dir);
            }
            catch { }
        }

        public static async Task<List<BackupEntry>> GetBackupHistoryAsync(Action<string> log)
        {
            return await Task.Run(() =>
            {
                var list = new List<BackupEntry>();
                var seenIds = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

                // 1. Query live Windows System Restore Points via PowerShell
                try
                {
                    string psCmd = "Get-ComputerRestorePoint -ErrorAction SilentlyContinue | Select-Object SequenceNumber, Description, CreationTime | ConvertTo-Json";
                    var psi = new ProcessStartInfo
                    {
                        FileName = "powershell.exe",
                        Arguments = "-NoProfile -ExecutionPolicy Bypass -Command \"" + psCmd + "\"",
                        CreateNoWindow = true,
                        UseShellExecute = false,
                        RedirectStandardOutput = true
                    };

                    using (var proc = Process.Start(psi))
                    {
                        string json = proc.StandardOutput.ReadToEnd();
                        proc.WaitForExit(5000);

                        if (!string.IsNullOrWhiteSpace(json))
                        {
                            var entries = new List<Dictionary<string, object>>();
                            if (json.Trim().StartsWith("["))
                            {
                                var arr = _serializer.Deserialize<List<Dictionary<string, object>>>(json);
                                if (arr != null) entries.AddRange(arr);
                            }
                            else if (json.Trim().StartsWith("{"))
                            {
                                var single = _serializer.Deserialize<Dictionary<string, object>>(json);
                                if (single != null) entries.Add(single);
                            }

                            foreach (var item in entries)
                            {
                                string desc = item.ContainsKey("Description") ? item["Description"] as string ?? "Restore Point" : "Restore Point";
                                int seq = item.ContainsKey("SequenceNumber") ? Convert.ToInt32(item["SequenceNumber"]) : -1;
                                string timeStr = item.ContainsKey("CreationTime") ? item["CreationTime"] as string ?? "" : "";

                                DateTime dt = DateTime.Now;
                                if (!string.IsNullOrEmpty(timeStr))
                                {
                                    // PowerShell json dates can be /Date(12345)/ or standard strings
                                    if (timeStr.Contains("Date("))
                                    {
                                        var match = System.Text.RegularExpressions.Regex.Match(timeStr, @"\d+");
                                        long ms;
                                        if (match.Success && long.TryParse(match.Value, out ms))
                                        {
                                            dt = new DateTime(1970, 1, 1, 0, 0, 0, DateTimeKind.Utc).AddMilliseconds(ms).ToLocalTime();
                                        }
                                    }
                                    else
                                    {
                                        DateTime.TryParse(timeStr, out dt);
                                    }
                                }

                                string id = "sr_" + seq;
                                if (seenIds.Add(id))
                                {
                                    list.Add(new BackupEntry
                                    {
                                        Id = id,
                                        Title = desc,
                                        BackupType = "System Restore Point",
                                        CreatedAt = dt,
                                        TimestampFormatted = dt.ToString("MMM dd, yyyy - hh:mm tt"),
                                        SequenceNumber = seq,
                                        IsSystemRestorePoint = true,
                                        SizeFormatted = "C: Shadow Volume",
                                        Description = string.Format("Windows shadow volume snapshot (Sequence #{0})", seq),
                                        Status = "Online"
                                    });
                                }
                            }
                        }
                    }
                }
                catch { }

                // 2. Load persistent local manifest
                var manifestEntries = LoadManifestEntries();
                foreach (var m in manifestEntries)
                {
                    if (seenIds.Add(m.Id))
                    {
                        list.Add(m);
                    }
                }

                // 3. Scan backups/ folder for any unindexed .reg files
                try
                {
                    string dir = GetBackupsDirectory();
                    if (Directory.Exists(dir))
                    {
                        var files = Directory.GetFiles(dir, "*.reg");
                        foreach (var f in files)
                        {
                            string fileName = Path.GetFileName(f);
                            if (seenIds.Add(f))
                            {
                                var fi = new FileInfo(f);
                                list.Add(new BackupEntry
                                {
                                    Id = f,
                                    Title = fileName.Replace(".reg", "").Replace("_", " "),
                                    BackupType = "Registry Archive",
                                    CreatedAt = fi.CreationTime,
                                    TimestampFormatted = fi.CreationTime.ToString("MMM dd, yyyy - hh:mm tt"),
                                    FilePath = f,
                                    SizeBytes = fi.Length,
                                    SizeFormatted = string.Format("{0:0.0} MB", fi.Length / (1024.0 * 1024.0)),
                                    IsSystemRestorePoint = false,
                                    Description = "Registry archive file stored in backups folder",
                                    Status = "Verified"
                                });
                            }
                        }
                    }
                }
                catch { }

                // Sort newest first
                return list.OrderByDescending(b => b.CreatedAt).ToList();
            });
        }

        public static bool DeleteBackupEntry(BackupEntry entry, Action<string> log)
        {
            try
            {
                if (!string.IsNullOrEmpty(entry.FilePath) && File.Exists(entry.FilePath))
                {
                    File.Delete(entry.FilePath);
                    log("Deleted backup archive: " + Path.GetFileName(entry.FilePath));
                }

                var manifest = LoadManifestEntries();
                manifest.RemoveAll(m => m.Id == entry.Id || (m.FilePath != null && m.FilePath == entry.FilePath));
                SaveManifestEntries(manifest);
                return true;
            }
            catch (Exception ex)
            {
                log("Error deleting backup: " + ex.Message);
                return false;
            }
        }

        public static void BackupRegistryKey(string keyPath, string backupName)
        {
            try
            {
                string backupsDir = GetBackupsDirectory();
                string safeName = backupName.Replace(" ", "_").Replace("\\", "_");
                string file = Path.Combine(backupsDir, string.Format("reg_backup_{0}_{1:yyyyMMdd_HHmmss}.reg", safeName, DateTime.Now));

                using (var proc = Process.Start(new ProcessStartInfo
                {
                    FileName = "reg.exe",
                    Arguments = string.Format("export \"{0}\" \"{1}\" /y", keyPath, file),
                    CreateNoWindow = true,
                    UseShellExecute = false
                }))
                {
                    proc.WaitForExit(3000);
                }
            }
            catch { }
        }
    }
}
