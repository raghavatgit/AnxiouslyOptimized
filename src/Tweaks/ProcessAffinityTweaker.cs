using System;
using System.Diagnostics;

namespace AnxiouslyOptimized.Tweaks
{
    public static class ProcessAffinityTweaker
    {
        public static void PinToPerformanceCores(string processName, long affinityMask)
        {
            foreach (var proc in Process.GetProcessesByName(processName))
            {
                proc.ProcessorAffinity = (IntPtr)affinityMask;
                proc.PriorityClass = ProcessPriorityClass.High;
            }
        }
    }
}
