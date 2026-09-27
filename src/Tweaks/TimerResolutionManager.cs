using System;
using System.Runtime.InteropServices;

namespace AnxiouslyOptimized.Tweaks
{
    public static class TimerResolutionManager
    {
        [DllImport("ntdll.dll", SetLastError = true)]
        private static extern int NtSetTimerResolution(uint DesiredResolution, bool SetResolution, out uint CurrentResolution);

        public static uint SetHighPrecisionResolution(uint desired100ns = 5000)
        {
            NtSetTimerResolution(desired100ns, true, out uint actual);
            return actual;
        }
    }
}
