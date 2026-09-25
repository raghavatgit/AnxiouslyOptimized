using System;
using System.Collections.Generic;

namespace AnxiouslyOptimized.Models
{
    public class PresetConfig
    {
        public string Title { get; set; }
        public string Description { get; set; }
        public List<string> TweakIds { get; set; }

        public PresetConfig()
        {
            TweakIds = new List<string>();
        }
    }
}
