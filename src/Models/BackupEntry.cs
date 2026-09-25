using System;

namespace AnxiouslyOptimized.Models
{
    public class BackupEntry
    {
        public string Id { get; set; }
        public string Title { get; set; }
        public string BackupType { get; set; } // "System Restore Point", "Registry Archive", "Automatic Snapshot"
        public DateTime CreatedAt { get; set; }
        public string TimestampFormatted { get; set; }
        public string FilePath { get; set; }
        public long SizeBytes { get; set; }
        public string SizeFormatted { get; set; }
        public int SequenceNumber { get; set; }
        public bool IsSystemRestorePoint { get; set; }
        public string Description { get; set; }
        public string Status { get; set; } // "Verified", "Protected", "Available"

        public BackupEntry()
        {
            Id = Guid.NewGuid().ToString("N");
            CreatedAt = DateTime.Now;
            SequenceNumber = -1;
            Status = "Protected";
            SizeFormatted = "Local Archive";
        }
    }
}
