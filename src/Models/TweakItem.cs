using System;
using System.ComponentModel;

namespace AnxiouslyOptimized.Models
{
    public class TweakItem : INotifyPropertyChanged
    {
        public string id { get; set; }
        public string title { get; set; }
        public string category { get; set; }
        public string impact { get; set; }
        public string description { get; set; }
        public string checkScript { get; set; }
        public string applyScript { get; set; }
        public string revertScript { get; set; }

        private bool _isApplied;
        public bool IsApplied
        {
            get { return _isApplied; }
            set
            {
                if (_isApplied != value)
                {
                    _isApplied = value;
                    OnPropertyChanged("IsApplied");
                    OnPropertyChanged("StatusBadgeText");
                    OnPropertyChanged("StatusBadgeType");
                }
            }
        }

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

        private bool _isBusy;
        public bool IsBusy
        {
            get { return _isBusy; }
            set
            {
                if (_isBusy != value)
                {
                    _isBusy = value;
                    OnPropertyChanged("IsBusy");
                }
            }
        }

        public string StatusBadgeText
        {
            get { return IsApplied ? "OPTIMIZED" : "STANDARD"; }
        }

        public string StatusBadgeType
        {
            get { return IsApplied ? "Optimized" : "Standard"; }
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
