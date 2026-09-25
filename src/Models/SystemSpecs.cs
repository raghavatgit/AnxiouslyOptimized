using System;

namespace AnxiouslyOptimized.Models
{
    public class SystemSpecs
    {
        public string CpuName { get; set; }
        public string RamSummary { get; set; }
        public string GpuName { get; set; }
        public string OsName { get; set; }
        public string EditionBadge { get; set; }
        public int BuildNumber { get; set; }
        public bool IsWin11 { get; set; }
        public string StorageSummary { get; set; }
        public int OptimizationScore { get; set; }
    }
}
