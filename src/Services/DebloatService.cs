using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using AnxiouslyOptimized.Models;

namespace AnxiouslyOptimized.Services
{
    public static class DebloatService
    {
        private static JavaScriptSerializer _serializer = new JavaScriptSerializer { MaxJsonLength = int.MaxValue };

        public static async Task<List<BloatPackage>> ScanInstalledBloatAsync(Action<string> log)
        {
            return await Task.Run(() =>
            {
                log("Scanning installed Windows Universal Apps and telemetry components...");
                var list = new List<BloatPackage>();

                // Known Safe Bloatware Catalog
                var catalog = new Dictionary<string, Tuple<string, string, bool>>(StringComparer.OrdinalIgnoreCase)
                {
                    { "Microsoft.BingWeather", Tuple.Create("Bing Weather", "MSN Weather app and live tile components", true) },
                    { "Microsoft.BingNews", Tuple.Create("Bing News", "MSN News and feed aggregator", true) },
                    { "Microsoft.GetHelp", Tuple.Create("Get Help", "Windows Contact Support and diagnostics assistant", true) },
                    { "Microsoft.Getstarted", Tuple.Create("Tips (Get Started)", "Windows onboarding tips and recommendations", true) },
                    { "Microsoft.MicrosoftSolitaireCollection", Tuple.Create("Solitaire Collection", "Casual games bundle with telemetry", true) },
                    { "Microsoft.People", Tuple.Create("Microsoft People", "Legacy Windows contact book integration", true) },
                    { "Microsoft.Todos", Tuple.Create("Microsoft To-Do", "Task manager app", true) },
                    { "Microsoft.YourPhone", Tuple.Create("Phone Link", "Android/iOS device sync link service", true) },
                    { "Microsoft.ZuneVideo", Tuple.Create("Films & TV", "Default legacy media player", true) },
                    { "Microsoft.ZuneMusic", Tuple.Create("Groove Music / Media Player", "Default audio media player", true) },
                    { "Microsoft.WindowsFeedbackHub", Tuple.Create("Feedback Hub", "Windows feedback reporting and user telemetry", true) },
                    { "Microsoft.WindowsMaps", Tuple.Create("Windows Maps", "Offline mapping and route service", true) },
                    { "Microsoft.PowerAutomateDesktop", Tuple.Create("Power Automate", "Workflow desktop automation utility", true) },
                    { "MicrosoftTeams", Tuple.Create("Microsoft Teams", "Personal consumer Teams chat integration", true) },
                    { "Microsoft.GamingApp", Tuple.Create("Xbox App", "Xbox PC gaming hub", false) },
                    { "Microsoft.XboxGamingOverlay", Tuple.Create("Xbox Game Bar", "In-game overlay and recording suite", false) }
                };

                try
                {
                    var psi = new ProcessStartInfo
                    {
                        FileName = "powershell.exe",
                        Arguments = "-NoProfile -ExecutionPolicy Bypass -Command \"Get-AppxPackage -AllUsers | Select-Object -ExpandProperty Name\"",
                        CreateNoWindow = true,
                        UseShellExecute = false,
                        RedirectStandardOutput = true
                    };

                    using (var proc = Process.Start(psi))
                    {
                        string line;
                        var installedSet = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
                        while ((line = proc.StandardOutput.ReadLine()) != null)
                        {
                            if (!string.IsNullOrWhiteSpace(line))
                                installedSet.Add(line.Trim());
                        }
                        proc.WaitForExit();

                        foreach (var kvp in catalog)
                        {
                            bool isInstalled = installedSet.Contains(kvp.Key);
                            if (isInstalled)
                            {
                                list.Add(new BloatPackage
                                {
                                    PackageName = kvp.Key,
                                    DisplayName = kvp.Value.Item1,
                                    Description = kvp.Value.Item2,
                                    IsSafe = kvp.Value.Item3,
                                    IsInstalled = true,
                                    IsSelected = kvp.Value.Item3 // Auto-select safe bloatware
                                });
                            }
                        }
                    }
                }
                catch (Exception ex)
                {
                    log("Debloat scan error: " + ex.Message);
                }

                log(string.Format("Found {0} removable universal apps on system.", list.Count));
                return list;
            });
        }

        public static async Task<int> RemoveSelectedBloatAsync(IEnumerable<BloatPackage> packages, Action<string> log)
        {
            return await Task.Run(() =>
            {
                var toRemove = new List<BloatPackage>();
                foreach (var p in packages)
                {
                    if (p.IsSelected) toRemove.Add(p);
                }

                if (toRemove.Count == 0) return 0;

                int removed = 0;
                foreach (var pkg in toRemove)
                {
                    log(string.Format("Removing {0} ({1})...", pkg.DisplayName, pkg.PackageName));
                    try
                    {
                        var sb = new System.Text.StringBuilder();
                        sb.AppendLine(string.Format("$name = '{0}'", pkg.PackageName));
                        // 1. Remove from all user profiles
                        sb.AppendLine("try { Get-AppxPackage -Name $name -AllUsers -ErrorAction SilentlyContinue | Remove-AppxPackage -AllUsers -ErrorAction SilentlyContinue } catch { }");
                        // 2. Remove from active user profile
                        sb.AppendLine("try { Get-AppxPackage -Name $name -ErrorAction SilentlyContinue | Remove-AppxPackage -ErrorAction SilentlyContinue } catch { }");
                        // 3. De-provision from online Windows image (prevents Windows 11 staging and automatic reinstallation)
                        sb.AppendLine("try { Get-AppxProvisionedPackage -Online -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -eq $name -or $_.PackageName -like ('*' + $name + '*') } | Remove-AppxProvisionedPackage -Online -ErrorAction SilentlyContinue | Out-Null } catch { }");
                        // 4. Verify whether the package was eliminated
                        sb.AppendLine("$allRemaining = Get-AppxPackage -Name $name -AllUsers -ErrorAction SilentlyContinue");
                        sb.AppendLine("if (-not $allRemaining) {");
                        sb.AppendLine("    Write-Host 'AO_DEBLOAT_STATUS:REMOVED_ALL'");
                        sb.AppendLine("} else {");
                        sb.AppendLine("    $userRemaining = Get-AppxPackage -Name $name -ErrorAction SilentlyContinue");
                        sb.AppendLine("    if (-not $userRemaining) {");
                        sb.AppendLine("        Write-Host 'AO_DEBLOAT_STATUS:REMOVED_USER'");
                        sb.AppendLine("    } else {");
                        sb.AppendLine("        Write-Host 'AO_DEBLOAT_STATUS:FAILED'");
                        sb.AppendLine("    }");
                        sb.AppendLine("}");

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
                            proc.StandardInput.WriteLine(sb.ToString());
                            proc.StandardInput.Close();

                            string outText = proc.StandardOutput.ReadToEnd();
                            proc.WaitForExit(30000);

                            if (outText.Contains("AO_DEBLOAT_STATUS:REMOVED_ALL"))
                            {
                                pkg.IsInstalled = false;
                                pkg.IsSelected = false;
                                removed++;
                                log(string.Format("  [OK] Removed {0} (system-wide and de-provisioned)", pkg.DisplayName));
                            }
                            else if (outText.Contains("AO_DEBLOAT_STATUS:REMOVED_USER"))
                            {
                                pkg.IsInstalled = false;
                                pkg.IsSelected = false;
                                removed++;
                                log(string.Format("  [OK] Removed {0} (active user profile)", pkg.DisplayName));
                            }
                            else
                            {
                                log(string.Format("  [WARN] Windows protected package {0} could not be removed.", pkg.DisplayName));
                            }
                        }
                    }
                    catch (Exception ex)
                    {
                        log(string.Format("  [ERR] {0}: {1}", pkg.DisplayName, ex.Message));
                    }
                }

                log(string.Format("Debloat completed. {0} packages removed.", removed));
                return removed;
            });
        }
    }
}
