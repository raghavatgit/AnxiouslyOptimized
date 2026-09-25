using System;
using System.ComponentModel;

namespace AnxiouslyOptimized.Models
{
    public class BloatPackage : INotifyPropertyChanged
    {
        public string PackageName { get; set; }
        public string DisplayName { get; set; }
        public string Description { get; set; }
        public string Category { get; set; }
        public bool IsSafe { get; set; }
        public bool IsInstalled { get; set; }

        private bool _isSelected;
        public bool IsSelected
        {
            get { return _isSelected; }
            set
            {
                if (_isSelected != value)
                {
                    _isSelected = value;
                    OnPropertyChanged("IsSelected");
                }
            }
        }

        public string SafetyBadge
        {
            get { return IsSafe ? "SAFE TO REMOVE" : "SYSTEM UTILITY"; }
        }

        public event PropertyChangedEventHandler PropertyChanged;
        protected virtual void OnPropertyChanged(string propertyName)
        {
            if (PropertyChanged != null)
            {
                PropertyChanged(this, new PropertyChangedEventArgs(propertyName));
            }
        }
    }
}
