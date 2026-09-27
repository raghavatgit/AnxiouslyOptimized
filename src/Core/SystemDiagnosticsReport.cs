using System;

namespace AnxiouslyOptimized.Core
{
    public class SystemDiagnosticsReport
    {
        public string ProcessorName { get; set; } = Environment.GetEnvironmentVariable("PROCESSOR_IDENTIFIER") ?? "Unknown";
        public int LogicalCores { get; set; } = Environment.ProcessorCount;
        public long SystemPageSize { get; set; } = Environment.SystemPageSize;
        public string OSVersion { get; set; } = Environment.OSVersion.ToString();
    }
}
