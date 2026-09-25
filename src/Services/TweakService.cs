using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Reflection;
using System.Text;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using AnxiouslyOptimized.Models;

namespace AnxiouslyOptimized.Services
{
    public static class TweakService
    {
        private static JavaScriptSerializer _serializer = new JavaScriptSerializer { MaxJsonLength = int.MaxValue };

        public static List<TweakItem> LoadTweaks(bool isWin11)
        {
            string fileName = isWin11 ? "tweaks_win11.json" : "tweaks_win10.json";
            string content = null;

            // 1. Try local file in config/ or src/Assets/
            string localPath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "config", fileName);
            if (!File.Exists(localPath))
                localPath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, fileName);

            if (File.Exists(localPath))
            {
                try { content = File.ReadAllText(localPath); } catch { }
            }

            // 2. Fall back to embedded resource
            if (string.IsNullOrEmpty(content))
            {
                try
                {
                    var asm = Assembly.GetExecutingAssembly();
                    string resName = "AnxiouslyOptimized.Assets." + fileName;
                    using (var stream = asm.GetManifestResourceStream(resName))
                    {
                        if (stream != null)
                        {
                            using (var reader = new StreamReader(stream))
                            {
                                content = reader.ReadToEnd();
                            }
                        }
                    }
                }
                catch { }
            }

            if (string.IsNullOrEmpty(content))
                return new List<TweakItem>();

            try
            {
                return _serializer.Deserialize<List<TweakItem>>(content) ?? new List<TweakItem>();
            }
            catch
            {
                return new List<TweakItem>();
            }
        }

        public static Dictionary<string, PresetConfig> LoadPresets()
        {
            string content = null;
            string localPath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "config", "presets.json");
            if (File.Exists(localPath))
            {
                try { content = File.ReadAllText(localPath); } catch { }
            }

            if (string.IsNullOrEmpty(content))
            {
                try
                {
                    var asm = Assembly.GetExecutingAssembly();
                    string resName = "AnxiouslyOptimized.Assets.presets.json";
                    using (var stream = asm.GetManifestResourceStream(resName))
                    {
                        if (stream != null)
                        {
                            using (var reader = new StreamReader(stream))
                            {
                                content = reader.ReadToEnd();
                            }
                        }
                    }
                }
                catch { }
            }

            var result = new Dictionary<string, PresetConfig>(StringComparer.OrdinalIgnoreCase);
            if (string.IsNullOrEmpty(content)) return result;

            try
            {
                var dict = _serializer.Deserialize<Dictionary<string, object>>(content);
                if (dict != null)
                {
                    foreach (var kvp in dict)
                    {
                        var pDict = kvp.Value as Dictionary<string, object>;
                        if (pDict != null)
                        {
                            var preset = new PresetConfig();
                            if (pDict.ContainsKey("title")) preset.Title = pDict["title"] as string;
                            else if (pDict.ContainsKey("name")) preset.Title = pDict["name"] as string;
                            if (pDict.ContainsKey("description")) preset.Description = pDict["description"] as string;
                            if (pDict.ContainsKey("tweaks"))
                            {
                                var twList = pDict["tweaks"] as System.Collections.ArrayList;
                                if (twList != null)
                                {
                                    foreach (var t in twList)
                                        preset.TweakIds.Add(t.ToString());
                                }
                            }
                            result[kvp.Key] = preset;

                            // Register friendly aliases for Quick Boost preset triggers
                            if (string.Equals(kvp.Key, "safe_baseline", StringComparison.OrdinalIgnoreCase))
                                result["safe"] = preset;
                            else if (string.Equals(kvp.Key, "gaming_rig", StringComparison.OrdinalIgnoreCase))
                                result["gaming"] = preset;
                            else if (string.Equals(kvp.Key, "emulator_pro", StringComparison.OrdinalIgnoreCase))
                                result["emulator"] = preset;
                        }
                    }
                }
            }
            catch { }

            return result;
        }

        public static async Task AuditTweaksAsync(IEnumerable<TweakItem> tweaks, Action<string, bool> onTweakAudited, Action<string> log)
        {
            await Task.Run(() =>
            {
                log("Auditing tweaks natively in-process (Win32 Registry Core Engine)...");

                var fallbackTweaks = new List<TweakItem>();
                int nativeCount = 0;

                foreach (var tw in tweaks)
                {
                    if (NativeTweakEngine.CanHandleNatively(tw.id))
                    {
                        bool isApplied;
                        if (NativeTweakEngine.CheckTweakNative(tw.id, out isApplied))
                        {
                            if (onTweakAudited != null)
                                onTweakAudited(tw.id, isApplied);
                            nativeCount++;
                            continue;
                        }
                    }

                    if (!string.IsNullOrEmpty(tw.checkScript))
                    {
                        fallbackTweaks.Add(tw);
                    }
                }

                if (fallbackTweaks.Count == 0)
                {
                    log(string.Format("Tweak audit completed in-memory: {0} checks evaluated natively (0 subprocesses spawned).", nativeCount));
                    return;
                }

                log(string.Format("Native audit processed {0} tweaks. Auditing {1} fallback tweaks via PowerShell...", nativeCount, fallbackTweaks.Count));

                var sbScript = new StringBuilder();
                sbScript.AppendLine("$results = @{}");

                foreach (var tw in fallbackTweaks)
                {
                    string safeId = tw.id;
                    sbScript.AppendLine(string.Format("try {{ $r = [bool]({0}); $results['{1}'] = $r }} catch {{ $results['{1}'] = $false }}", tw.checkScript, safeId));
                }

                sbScript.AppendLine("foreach ($k in $results.Keys) { Write-Host \"AO_RESULT:$($k):$($results[$k])\" }");

                try
                {
                    var psi = new ProcessStartInfo
                    {
                        FileName = "powershell.exe",
                        Arguments = "-NoProfile -ExecutionPolicy Bypass -Command -",
                        CreateNoWindow = true,
                        UseShellExecute = false,
                        RedirectStandardInput = true,
                        RedirectStandardOutput = true
                    };

                    using (var proc = Process.Start(psi))
                    {
                        proc.StandardInput.WriteLine(sbScript.ToString());
                        proc.StandardInput.Close();

                        string line;
                        while ((line = proc.StandardOutput.ReadLine()) != null)
                        {
                            if (line.StartsWith("AO_RESULT:"))
                            {
                                var parts = line.Split(':');
                                if (parts.Length >= 3)
                                {
                                    string id = parts[1];
                                    bool isApplied = parts[2].Trim().Equals("True", StringComparison.OrdinalIgnoreCase);
                                    if (onTweakAudited != null)
                                        onTweakAudited(id, isApplied);
                                }
                            }
                        }
                        proc.WaitForExit();
                    }
                }
                catch (Exception ex)
                {
                    log("Tweak audit error: " + ex.Message);
                }

                log("Tweak audit completed.");
            });
        }

        public static async Task<bool> ExecuteTweakActionAsync(TweakItem tweak, bool apply, Action<string> log)
        {
            return await Task.Run(() =>
            {
                string action = apply ? "Applying" : "Reverting";

                if (NativeTweakEngine.CanHandleNatively(tweak.id))
                {
                    log(string.Format("{0} tweak: {1} (Native In-Process Win32 Core)...", action, tweak.title));
                    try
                    {
                        bool nativeOk = apply
                            ? NativeTweakEngine.ApplyTweakNative(tweak.id, log)
                            : NativeTweakEngine.RevertTweakNative(tweak.id, log);

                        if (nativeOk)
                        {
                            log(string.Format("  [OK] {0} ({1} completed via Native Win32 API in RAM)", tweak.title, action));
                            return true;
                        }
                        else
                        {
                            log(string.Format("  [WARN] Native execution did not succeed for {0}, falling back to PowerShell script...", tweak.title));
                        }
                    }
                    catch (Exception ex)
                    {
                        log(string.Format("  [WARN] Native execution error for {0}: {1}. Falling back to PowerShell...", tweak.title, ex.Message));
                    }
                }

                string script = apply ? tweak.applyScript : tweak.revertScript;
                if (string.IsNullOrEmpty(script)) return false;

                log(string.Format("{0} tweak: {1} (PowerShell Fallback)...", action, tweak.title));

                try
                {
                    var psi = new ProcessStartInfo
                    {
                        FileName = "powershell.exe",
                        Arguments = "-NoProfile -ExecutionPolicy Bypass -Command \"" + script.Replace("\"", "\\\"") + "\"",
                        CreateNoWindow = true,
                        UseShellExecute = false,
                        RedirectStandardOutput = true,
                        RedirectStandardError = true
                    };

                    using (var proc = Process.Start(psi))
                    {
                        proc.WaitForExit(8000);
                        if (proc.ExitCode == 0)
                        {
                            log(string.Format("  [OK] {0} ({1} completed via PowerShell)", tweak.title, action));
                            return true;
                        }
                        else
                        {
                            string err = proc.StandardError.ReadToEnd();
                            log(string.Format("  [ERR] {0} failed: {1}", tweak.title, err));
                            return false;
                        }
                    }
                }
                catch (Exception ex)
                {
                    log(string.Format("  [ERR] {0}: {1}", tweak.title, ex.Message));
                    return false;
                }
            });
        }
    }
}
