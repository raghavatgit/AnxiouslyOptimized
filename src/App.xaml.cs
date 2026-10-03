using System;
using System.Threading;
using System.Windows;
using System.Windows.Threading;
using AnxiouslyOptimized.Services;

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

            // H-2 fix: three separate crash entry-points all converge here.
            // The 1ms winmm timer is a GLOBAL system resource - if we crash without releasing it,
            // every process on the machine runs with 1ms scheduling granularity until reboot.
            AppDomain.CurrentDomain.UnhandledException += (s, args) =>
            {
                SafeShutdownDaemon();
                try
                {
                    var ex = args.ExceptionObject as Exception;
                    MessageBox.Show("An unexpected issue occurred: " + (ex != null ? ex.Message : "Unknown"), "AnxiouslyOptimized", MessageBoxButton.OK, MessageBoxImage.Warning);
                }
                catch { }
            };

            // H-2 fix: WPF-thread exceptions (e.g. unhandled in UI event handlers)
            DispatcherUnhandledException += (s, args) =>
            {
                SafeShutdownDaemon();
                try
                {
                    MessageBox.Show("An unexpected UI error occurred: " + args.Exception.Message, "AnxiouslyOptimized", MessageBoxButton.OK, MessageBoxImage.Warning);
                }
                catch { }
                args.Handled = true;
            };

            // H-2 fix: async Task exceptions that are never awaited
            System.Threading.Tasks.TaskScheduler.UnobservedTaskException += (s, args) =>
            {
                args.SetObserved();
                // Only log; do not shut down the daemon for minor async exceptions
                System.Diagnostics.Debug.WriteLine("UnobservedTaskException: " + args.Exception.Message);
            };

            base.OnStartup(e);
        }

        protected override void OnExit(ExitEventArgs e)
        {
            SafeShutdownDaemon();
            if (_mutex != null)
            {
                _mutex.ReleaseMutex();
                _mutex.Dispose();
            }
            base.OnExit(e);
        }

        /// <summary>H-2: Idempotent daemon stop - safe to call multiple times from any crash path.</summary>
        private static void SafeShutdownDaemon()
        {
            try
            {
                if (ActiveGameModeDaemon.IsEnabled)
                    ActiveGameModeDaemon.StopDaemon();
            }
            catch { }
        }
    }
}
