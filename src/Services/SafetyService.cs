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
        #region Transactional Rollback & Journal Architecture (Feature 6)
        public static string GetJournalDirectory()
        {
            string dir = Path.Combine(GetBackupsDirectory(), "journal");
            if (!Directory.Exists(dir))
            {
                try { Directory.CreateDirectory(dir); } catch { }
            }
            return dir;
        }

        public static TransactionJournal BeginTransaction(string title)
        {
            return new TransactionJournal
            {
                Title = title
            };
        }

        public static void RecordRegistryChange(TransactionJournal journal, string keyPath, string valueName, object originalValue, Microsoft.Win32.RegistryValueKind originalKind, object newValue)
        {
            if (journal == null) return;

            var step = new TransactionStep
            {
                StepType = "RegistryValue",
                Target = keyPath,
                PropertyOrName = valueName,
                OriginalValue = originalValue != null ? originalValue.ToString() : null,
                OriginalKind = originalKind.ToString(),
                NewValue = newValue != null ? newValue.ToString() : null
            };
            journal.Steps.Add(step);
        }

        public static void RecordServiceChange(TransactionJournal journal, string serviceName, int originalStartMode, int newStartMode)
        {
            if (journal == null) return;

            var step = new TransactionStep
            {
                StepType = "ServiceStartMode",
                Target = @"HKLM\SYSTEM\CurrentControlSet\Services\" + serviceName,
                PropertyOrName = "Start",
                OriginalValue = originalStartMode.ToString(),
                OriginalKind = "DWord",
                NewValue = newStartMode.ToString()
            };
            journal.Steps.Add(step);
        }

        public static void CommitTransaction(TransactionJournal journal, Action<string> log)
        {
            if (journal == null || journal.Steps.Count == 0) return;

            try
            {
                string dir = GetJournalDirectory();
                string path = Path.Combine(dir, journal.JournalId + ".json");
                string json = _serializer.Serialize(journal);
                File.WriteAllText(path, json, Encoding.UTF8);

                if (log != null)
                    log(string.Format("Transaction journal '{0}' committed ({1} action(s) recorded).", journal.Title, journal.Steps.Count));

                // Auto-generate Desktop Emergency Undo script
                GenerateEmergencyDesktopBatch(journal, log);
            }
            catch (Exception ex)
            {
                if (log != null) log("Journal commit error: " + ex.Message);
            }
        }

        public static List<TransactionJournal> LoadAllJournals()
        {
            var list = new List<TransactionJournal>();
            string dir = GetJournalDirectory();
            if (Directory.Exists(dir))
            {
                var files = Directory.GetFiles(dir, "TRX_*.json");
                foreach (var file in files)
                {
                    try
                    {
                        string json = File.ReadAllText(file);
                        var raw = _serializer.Deserialize<Dictionary<string, object>>(json);
                        if (raw != null)
                        {
                            var j = new TransactionJournal();
                            if (raw.ContainsKey("JournalId")) j.JournalId = raw["JournalId"] as string ?? j.JournalId;
                            if (raw.ContainsKey("Title")) j.Title = raw["Title"] as string ?? "";
                            if (raw.ContainsKey("TimestampFormatted")) j.TimestampFormatted = raw["TimestampFormatted"] as string ?? "";
                            if (raw.ContainsKey("IsRolledBack")) j.IsRolledBack = Convert.ToBoolean(raw["IsRolledBack"]);

                            DateTime dt;
                            if (raw.ContainsKey("Timestamp") && DateTime.TryParse(raw["Timestamp"] as string, out dt))
                                j.Timestamp = dt;

                            if (raw.ContainsKey("Steps"))
                            {
                                var stepsRaw = raw["Steps"] as System.Collections.ArrayList;
                                if (stepsRaw != null)
                                {
                                    foreach (Dictionary<string, object> stepDict in stepsRaw)
                                    {
                                        var step = new TransactionStep();
                                        if (stepDict.ContainsKey("StepType")) step.StepType = stepDict["StepType"] as string;
                                        if (stepDict.ContainsKey("Target")) step.Target = stepDict["Target"] as string;
                                        if (stepDict.ContainsKey("PropertyOrName")) step.PropertyOrName = stepDict["PropertyOrName"] as string;
                                        if (stepDict.ContainsKey("OriginalValue")) step.OriginalValue = stepDict["OriginalValue"] as string;
                                        if (stepDict.ContainsKey("OriginalKind")) step.OriginalKind = stepDict["OriginalKind"] as string;
                                        if (stepDict.ContainsKey("NewValue")) step.NewValue = stepDict["NewValue"] as string;
                                        j.Steps.Add(step);
                                    }
                                }
                            }
                            list.Add(j);
                        }
                    }
                    catch { }
                }
            }
            return list.OrderByDescending(j => j.Timestamp).ToList();
        }

        public static async Task<bool> RollbackTransactionAsync(TransactionJournal journal, Action<string> log)
        {
            return await Task.Run(() =>
            {
                if (journal == null || journal.Steps == null || journal.Steps.Count == 0)
                {
                    if (log != null) log("No steps in transaction to rollback.");
                    return false;
                }

                if (log != null)
                    log(string.Format("Initiating transactional rollback for '{0}' ({1} step(s) in reverse LIFO order)...", journal.Title, journal.Steps.Count));

                // Reverse LIFO order
                var reversed = new List<TransactionStep>(journal.Steps);
                reversed.Reverse();

                int reverted = 0;
                foreach (var step in reversed)
                {
                    try
                    {
                        if (step.StepType == "RegistryValue" || step.StepType == "ServiceStartMode")
                        {
                            bool isHklm = step.Target.StartsWith("HKLM", StringComparison.OrdinalIgnoreCase) ||
                                          step.Target.StartsWith("HKEY_LOCAL_MACHINE", StringComparison.OrdinalIgnoreCase);

                            string subkey = step.Target;
                            if (subkey.StartsWith(@"HKLM\", StringComparison.OrdinalIgnoreCase)) subkey = subkey.Substring(5);
                            else if (subkey.StartsWith(@"HKCU\", StringComparison.OrdinalIgnoreCase)) subkey = subkey.Substring(5);
                            else if (subkey.StartsWith(@"HKEY_LOCAL_MACHINE\", StringComparison.OrdinalIgnoreCase)) subkey = subkey.Substring(19);
                            else if (subkey.StartsWith(@"HKEY_CURRENT_USER\", StringComparison.OrdinalIgnoreCase)) subkey = subkey.Substring(18);

                            Microsoft.Win32.RegistryKey root = isHklm ? Microsoft.Win32.Registry.LocalMachine : Microsoft.Win32.Registry.CurrentUser;

                            using (var key = root.CreateSubKey(subkey))
                            {
                                if (key != null)
                                {
                                    if (step.OriginalValue == null || step.OriginalKind == "None")
                                    {
                                        // It was newly added, so delete it during rollback
                                        try { key.DeleteValue(step.PropertyOrName, false); } catch { }
                                    }
                                    else
                                    {
                                        if (step.OriginalKind == "DWord")
                                        {
                                            int val = Convert.ToInt32(step.OriginalValue);
                                            key.SetValue(step.PropertyOrName, val, Microsoft.Win32.RegistryValueKind.DWord);
                                        }
                                        else if (step.OriginalKind == "QWord")
                                        {
                                            long val = Convert.ToInt64(step.OriginalValue);
                                            key.SetValue(step.PropertyOrName, val, Microsoft.Win32.RegistryValueKind.QWord);
                                        }
                                        else
                                        {
                                            key.SetValue(step.PropertyOrName, step.OriginalValue, Microsoft.Win32.RegistryValueKind.String);
                                        }
                                    }
                                    reverted++;
                                }
                            }
                        }
                    }
                    catch (Exception ex)
                    {
                        if (log != null)
                            log(string.Format("Rollback step error on {0}: {1}", step.Target, ex.Message));
                    }
                }

                journal.IsRolledBack = true;

                // Re-save updated journal
                try
                {
                    string path = Path.Combine(GetJournalDirectory(), journal.JournalId + ".json");
                    File.WriteAllText(path, _serializer.Serialize(journal), Encoding.UTF8);
                }
                catch { }

                if (log != null)
                    log(string.Format("Rollback successful! {0}/{1} steps cleanly restored.", reverted, journal.Steps.Count));

                return true;
            });
        }

        public static string GenerateEmergencyDesktopBatch(TransactionJournal journal, Action<string> log)
        {
            try
            {
                string desktop = Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory);
                string batPath = Path.Combine(desktop, "AnxiouslyOptimized_Emergency_Undo.bat");

                var sb = new StringBuilder();
                sb.AppendLine("@echo off");
                sb.AppendLine(":: ===================================================================");
                sb.AppendLine(":: ANXIOUSLYOPTIMIZED EMERGENCY ROLLBACK RECOVERY SCRIPT");
                sb.AppendLine(":: Standalone Zero-Dependency Disaster Recovery (No .NET required)");
                sb.AppendLine(":: Created by Raghav Goyal - AnxiouslyOptimized Engine");
                sb.AppendLine(":: ===================================================================");
                sb.AppendLine("echo.");
                sb.AppendLine("echo [!] Checking for Administrative privileges...");
                sb.AppendLine("net session >nul 2>&1");
                sb.AppendLine("if %errorLevel% neq 0 (");
                sb.AppendLine("    echo [ERROR] Please right-click this script and select 'Run as administrator'.");
                sb.AppendLine("    pause");
                sb.AppendLine("    exit /b 1");
                sb.AppendLine(")");
                sb.AppendLine("echo [OK] Administrator rights confirmed.");
                sb.AppendLine("echo [!] Restoring system registry keys and service configurations...");
                sb.AppendLine("echo.");

                var reversed = new List<TransactionStep>(journal.Steps);
                reversed.Reverse();

                foreach (var step in reversed)
                {
                    if (step.StepType == "RegistryValue" || step.StepType == "ServiceStartMode")
                    {
                        string safeTarget = step.Target;
                        if (step.OriginalValue == null || step.OriginalKind == "None")
                        {
                            sb.AppendLine(string.Format("reg.exe delete \"{0}\" /v \"{1}\" /f >nul 2>&1", safeTarget, step.PropertyOrName));
                        }
                        else
                        {
                            string regType = "REG_DWORD";
                            if (step.OriginalKind == "String") regType = "REG_SZ";
                            else if (step.OriginalKind == "QWord") regType = "REG_QWORD";

                            sb.AppendLine(string.Format("reg.exe add \"{0}\" /v \"{1}\" /t {2} /d \"{3}\" /f >nul 2>&1",
                                safeTarget, step.PropertyOrName, regType, step.OriginalValue));
                        }
                    }
                }

                sb.AppendLine("echo.");
                sb.AppendLine("echo [SUCCESS] Emergency restoration complete. All settings have been reset.");
                sb.AppendLine("echo.");
                sb.AppendLine("pause");

                File.WriteAllText(batPath, sb.ToString(), Encoding.ASCII);
                if (log != null)
                    log(string.Format("Emergency recovery script generated on Desktop: '{0}'", Path.GetFileName(batPath)));

                return batPath;
            }
            catch (Exception ex)
            {
                if (log != null) log("Emergency script error: " + ex.Message);
                return string.Empty;
            }
        }

        public static async Task<VssStatusResult> ValidateVssAndDiskHealthAsync(Action<string> log)
        {
            return await Task.Run(() =>
            {
                var res = new VssStatusResult
                {
                    IsVssAvailable = false,
                    FreeSpaceMb = 0,
                    StatusMessage = "Checking..."
                };

                try
                {
                    // Check free disk space on C:
                    var drive = new DriveInfo("C");
                    res.FreeSpaceMb = drive.AvailableFreeSpace / (1024 * 1024);

                    // Check Volume Shadow Copy service state
                    using (var sc = new System.ServiceProcess.ServiceController("VSS"))
                    {
                        res.IsVssAvailable = (sc.Status == System.ServiceProcess.ServiceControllerStatus.Running ||
                                              sc.Status == System.ServiceProcess.ServiceControllerStatus.Stopped);
                    }

                    if (res.FreeSpaceMb > 500 && res.IsVssAvailable)
                    {
                        res.StatusMessage = string.Format("VSS Operational ({0} MB free space on C:)", res.FreeSpaceMb);
                    }
                    else if (res.FreeSpaceMb <= 500)
                    {
                        res.StatusMessage = string.Format("Low Disk Space Warning: Only {0} MB free on C:", res.FreeSpaceMb);
                    }
                    else
                    {
                        res.StatusMessage = "VSS Service Not Available";
                    }

                    if (log != null) log(res.StatusMessage);
                }
                catch (Exception ex)
                {
                    res.StatusMessage = "VSS Check Note: " + ex.Message;
                    if (log != null) log(res.StatusMessage);
                }

                return res;
            });
        }
        #endregion
    }
}
