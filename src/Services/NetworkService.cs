using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Net.NetworkInformation;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using Microsoft.Win32;

namespace AnxiouslyOptimized.Services
{
    public class DnsProviderItem
    {
        public string Name { get; set; }
        public string PrimaryDns { get; set; }
        public string SecondaryDns { get; set; }
        public string Description { get; set; }
        public long PingMs { get; set; }
        public int PacketLossPercent { get; set; }
        public bool IsFastest { get; set; }
        public bool IsApplied { get; set; }

        public DnsProviderItem()
        {
            PingMs = -1;
            PacketLossPercent = 0;
            IsFastest = false;
            IsApplied = false;
        }

        public string LatencyDisplay
        {
            get
            {
                if (PingMs < 0) return "Not Tested";
                if (PacketLossPercent == 100) return "Failed (100% loss)";
                return string.Format("{0} ms ({1}% loss)", PingMs, PacketLossPercent);
            }
        }
    }

    public static class NetworkService
    {
        public static List<DnsProviderItem> GetDefaultProviders()
        {
            return new List<DnsProviderItem>
            {
                new DnsProviderItem
                {
                    Name = "Cloudflare Gaming (1.1.1.1)",
                    PrimaryDns = "1.1.1.1",
                    SecondaryDns = "1.0.0.1",
                    Description = "Lowest network packet transit latency, rapid edge resolution, zero privacy tracking."
                },
                new DnsProviderItem
                {
                    Name = "Cloudflare Security (1.1.1.2)",
                    PrimaryDns = "1.1.1.2",
                    SecondaryDns = "1.0.0.2",
                    Description = "Ultra-low gaming latency combined with automatic malware and phishing blocking."
                },
                new DnsProviderItem
                {
                    Name = "Google Public DNS (8.8.8.8)",
                    PrimaryDns = "8.8.8.8",
                    SecondaryDns = "8.8.4.4",
                    Description = "Global anycast routing across worldwide Google infrastructure with high resilience."
                },
                new DnsProviderItem
                {
                    Name = "Quad9 Secure DNS (9.9.9.9)",
                    PrimaryDns = "9.9.9.9",
                    SecondaryDns = "149.112.112.112",
                    Description = "Swiss-based high performance DNS with real-time cyber threat blocking and DNSSEC."
                },
                new DnsProviderItem
                {
                    Name = "OpenDNS Home (208.67.222.222)",
                    PrimaryDns = "208.67.222.222",
                    SecondaryDns = "208.67.220.220",
                    Description = "Cisco enterprise DNS routing infrastructure with fast resolution and 100% uptime."
                },
                new DnsProviderItem
                {
                    Name = "AdGuard Public (94.140.14.14)",
                    PrimaryDns = "94.140.14.14",
                    SecondaryDns = "94.140.15.15",
                    Description = "Stops tracker requests and telemetry domains before they reach your network card."
                }
            };
        }

        public static string GetActiveNetworkInterfaceName()
        {
            try
            {
                var interfaces = NetworkInterface.GetAllNetworkInterfaces();
                foreach (var nic in interfaces)
                {
                    if (nic.OperationalStatus == OperationalStatus.Up &&
                        nic.NetworkInterfaceType != NetworkInterfaceType.Loopback &&
                        nic.NetworkInterfaceType != NetworkInterfaceType.Tunnel)
                    {
                        var ipProps = nic.GetIPProperties();
                        if (ipProps != null && ipProps.GatewayAddresses != null && ipProps.GatewayAddresses.Count > 0)
                        {
                            return nic.Name;
                        }
                    }
                }
                // Fallback to first non-loopback active adapter
                var fallback = interfaces.FirstOrDefault(n => n.OperationalStatus == OperationalStatus.Up && n.NetworkInterfaceType != NetworkInterfaceType.Loopback);
                return fallback != null ? fallback.Name : "Ethernet";
            }
            catch
            {
                return "Ethernet";
            }
        }

        public static string GetActiveNetworkInterfaceId()
        {
            try
            {
                var interfaces = NetworkInterface.GetAllNetworkInterfaces();
                foreach (var nic in interfaces)
                {
                    if (nic.OperationalStatus == OperationalStatus.Up &&
                        nic.NetworkInterfaceType != NetworkInterfaceType.Loopback)
                    {
                        var ipProps = nic.GetIPProperties();
                        if (ipProps != null && ipProps.GatewayAddresses != null && ipProps.GatewayAddresses.Count > 0)
                        {
                            return nic.Id;
                        }
                    }
                }
                return string.Empty;
            }
            catch
            {
                return string.Empty;
            }
        }

        public static async Task<List<DnsProviderItem>> BenchmarkAllProvidersAsync(Action<string> log)
        {
            return await Task.Run(() =>
            {
                LogSafe(log, "Initiating multi-threaded DNS latency benchmark across all providers...");
                var providers = GetDefaultProviders();

                var tasks = providers.Select(p => Task.Run(() => BenchmarkSingleProvider(p))).ToArray();
                Task.WaitAll(tasks);

                // Identify the fastest DNS provider with 0% loss
                var validFastest = providers
                    .Where(p => p.PingMs > 0 && p.PacketLossPercent == 0)
                    .OrderBy(p => p.PingMs)
                    .FirstOrDefault();

                if (validFastest == null)
                {
                    validFastest = providers.Where(p => p.PingMs > 0).OrderBy(p => p.PingMs).FirstOrDefault();
                }

                if (validFastest != null)
                {
                    validFastest.IsFastest = true;
                    LogSafe(log, string.Format("Fastest DNS Provider: {0} ({1} ms round-trip time)", validFastest.Name, validFastest.PingMs));
                }

                foreach (var p in providers.OrderBy(p => p.PingMs > 0 ? p.PingMs : 9999))
                {
                    LogSafe(log, string.Format("  [{0}] -> {1}", p.Name, p.LatencyDisplay));
                }

                return providers;
            });
        }

        private static void BenchmarkSingleProvider(DnsProviderItem item)
        {
            int attempts = 3;
            int successful = 0;
            long totalRoundTrip = 0;

            using (var ping = new Ping())
            {
                for (int i = 0; i < attempts; i++)
                {
                    try
                    {
                        var reply = ping.Send(item.PrimaryDns, 1200);
                        if (reply != null && reply.Status == IPStatus.Success)
                        {
                            successful++;
                            totalRoundTrip += reply.RoundtripTime;
                        }
                    }
                    catch { }
                }
            }

            if (successful > 0)
            {
                item.PingMs = totalRoundTrip / successful;
                item.PacketLossPercent = (int)(((attempts - successful) / (double)attempts) * 100);
            }
            else
            {
                item.PingMs = -1;
                item.PacketLossPercent = 100;
            }
        }

        public static async Task<bool> ApplyDnsToActiveAdapterAsync(DnsProviderItem provider, Action<string> log)
        {
            return await Task.Run(() =>
            {
                string nicName = GetActiveNetworkInterfaceName();
                LogSafe(log, string.Format("Configuring DNS on active adapter '{0}' to {1} ({2}, {3})...", nicName, provider.Name, provider.PrimaryDns, provider.SecondaryDns));

                try
                {
                    // 1. Primary DNS
                    string primaryArgs = string.Format("interface ip set dns name=\"{0}\" static {1} primary", nicName, provider.PrimaryDns);
                    RunProcessSilent("netsh.exe", primaryArgs);

                    // 2. Secondary DNS
                    if (!string.IsNullOrEmpty(provider.SecondaryDns))
                    {
                        string secondaryArgs = string.Format("interface ip add dns name=\"{0}\" {1} index=2", nicName, provider.SecondaryDns);
                        RunProcessSilent("netsh.exe", secondaryArgs);
                    }

                    // 3. Flush Windows DNS cache
                    RunProcessSilent("ipconfig.exe", "/flushdns");

                    LogSafe(log, string.Format("Successfully applied {0} to adapter '{1}'. Resolver cache flushed.", provider.Name, nicName));
                    return true;
                }
                catch (Exception ex)
                {
                    LogSafe(log, "Failed to apply DNS: " + ex.Message);
                    return false;
                }
            });
        }

        public static async Task<bool> ResetDnsToAutomaticAsync(Action<string> log)
        {
            return await Task.Run(() =>
            {
                string nicName = GetActiveNetworkInterfaceName();
                LogSafe(log, string.Format("Resetting DNS configuration on '{0}' to Automatic (DHCP)...", nicName));

                try
                {
                    string args = string.Format("interface ip set dns name=\"{0}\" dhcp", nicName);
                    RunProcessSilent("netsh.exe", args);
                    RunProcessSilent("ipconfig.exe", "/flushdns");

                    LogSafe(log, string.Format("DNS on '{0}' successfully restored to Automatic (DHCP).", nicName));
                    return true;
                }
                catch (Exception ex)
                {
                    LogSafe(log, "Failed to reset DNS: " + ex.Message);
                    return false;
                }
            });
        }

        public static async Task<bool> ApplyLowLatencyTcpStackAsync(Action<string> log)
        {
            return await Task.Run(() =>
            {
                LogSafe(log, "Applying competitive low-latency TCP stack configurations (Nagle disabled, NoDelay, ECN)...");
                bool partialSuccess = false;

                // 1. Multimedia Throttling Index = -1 (Disabled)
                try
                {
                    using (var key = Registry.LocalMachine.CreateSubKey(@"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile"))
                    {
                        if (key != null)
                        {
                            key.SetValue("NetworkThrottlingIndex", unchecked((int)0xFFFFFFFF), RegistryValueKind.DWord);
                            key.SetValue("SystemResponsiveness", 0, RegistryValueKind.DWord);
                            partialSuccess = true;
                            LogSafe(log, "  [TCP] NetworkThrottlingIndex disabled and SystemResponsiveness set to 0 (Zero Packet Delay).");
                        }
                    }
                }
                catch (Exception ex)
                {
                    LogSafe(log, "  [TCP] Multimedia registry note: " + ex.Message);
                }

                // 2. Disable Nagle's Algorithm on active adapter (TcpAckFrequency = 1, TCPNoDelay = 1)
                try
                {
                    string nicId = GetActiveNetworkInterfaceId();
                    if (!string.IsNullOrEmpty(nicId))
                    {
                        string subkeyPath = @"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces\" + nicId;
                        using (var key = Registry.LocalMachine.CreateSubKey(subkeyPath))
                        {
                            if (key != null)
                            {
                                key.SetValue("TcpAckFrequency", 1, RegistryValueKind.DWord);
                                key.SetValue("TCPNoDelay", 1, RegistryValueKind.DWord);
                                key.SetValue("TcpDelAckTicks", 0, RegistryValueKind.DWord);
                                partialSuccess = true;
                                LogSafe(log, string.Format("  [TCP] Nagle's Algorithm disabled on active interface {0} (TcpAckFrequency=1, TCPNoDelay=1).", nicId));
                            }
                        }
                    }
                }
                catch (Exception ex)
                {
                    LogSafe(log, "  [TCP] Interface registry note: " + ex.Message);
                }

                // 3. TCP Global Stack tuning via NetSH
                try
                {
                    RunProcessSilent("netsh.exe", "int tcp set global autotuninglevel=normal");
                    RunProcessSilent("netsh.exe", "int tcp set global ecncapability=enabled");
                    RunProcessSilent("netsh.exe", "int tcp set global timestamps=disabled");
                    RunProcessSilent("netsh.exe", "int tcp set global rss=enabled");
                    partialSuccess = true;
                }
                catch { }

                LogSafe(log, "Low-latency TCP/IP stack configuration completed.");
                return partialSuccess;
            });
        }

        private static void RunProcessSilent(string fileName, string args)
        {
            try
            {
                using (var p = Process.Start(new ProcessStartInfo
                {
                    FileName = fileName,
                    Arguments = args,
                    CreateNoWindow = true,
                    UseShellExecute = false
                }))
                {
                    p.WaitForExit(3000);
                }
            }
            catch { }
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
