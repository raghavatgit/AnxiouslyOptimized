using System;
using System.Collections.Generic;

namespace AnxiouslyOptimized.Models
{
    public class TransactionStep
    {
        public string StepType { get; set; } // "RegistryValue", "RegistryKey", "ServiceStartMode", "NetworkDns"
        public string Target { get; set; }   // e.g. "HKLM\SYSTEM\CurrentControlSet\Services\DiagTrack"
        public string PropertyOrName { get; set; } // e.g. "Start" or "ValueName"
        public string OriginalValue { get; set; }  // string representation or null if did not exist
        public string OriginalKind { get; set; }   // "DWord", "String", "QWord", "None"
        public string NewValue { get; set; }

        public TransactionStep()
        {
            StepType = "RegistryValue";
            OriginalKind = "None";
        }
    }

    public class TransactionJournal
    {
        public string JournalId { get; set; }
        public string Title { get; set; }
        public DateTime Timestamp { get; set; }
        public string TimestampFormatted { get; set; }
        public List<TransactionStep> Steps { get; set; }
        public bool IsRolledBack { get; set; }

        public int TotalActions
        {
            get { return Steps != null ? Steps.Count : 0; }
        }

        public TransactionJournal()
        {
            JournalId = "TRX_" + DateTime.Now.ToString("yyyyMMdd_HHmmss");
            Timestamp = DateTime.Now;
            TimestampFormatted = DateTime.Now.ToString("MMM dd, yyyy - hh:mm tt");
            Steps = new List<TransactionStep>();
            IsRolledBack = false;
        }
    }

    public class VssStatusResult
    {
        public bool IsVssAvailable { get; set; }
        public long FreeSpaceMb { get; set; }
        public string StatusMessage { get; set; }
    }
}
