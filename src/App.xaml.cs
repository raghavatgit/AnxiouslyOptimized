using System;
using System.Threading;
using System.Windows;

namespace AnxiouslyOptimized
{
    public partial class App : Application
    {
        private static Mutex _mutex;

        protected override void OnStartup(StartupEventArgs e)
        {
            const string appName = "AnxiouslyOptimized_SingleInstance_Mutex";
            bool createdNew;

            _mutex = new Mutex(true, appName, out createdNew);

            if (!createdNew)
            {
                MessageBox.Show("AnxiouslyOptimized is already running.", "AnxiouslyOptimized", MessageBoxButton.OK, MessageBoxImage.Information);
                Shutdown();
                return;
            }

            AppDomain.CurrentDomain.UnhandledException += (s, args) =>
            {
                // Prevent crash without user notification
                try
                {
                    var ex = args.ExceptionObject as Exception;
                    MessageBox.Show("An unexpected issue occurred: " + (ex != null ? ex.Message : "Unknown"), "AnxiouslyOptimized", MessageBoxButton.OK, MessageBoxImage.Warning);
                }
                catch { }
            };

            base.OnStartup(e);
        }

        protected override void OnExit(ExitEventArgs e)
        {
            if (_mutex != null)
            {
                _mutex.ReleaseMutex();
                _mutex.Dispose();
            }
            base.OnExit(e);
        }
    }
}
