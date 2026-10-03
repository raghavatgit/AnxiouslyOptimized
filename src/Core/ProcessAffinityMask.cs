// feat(cpu): calculate 64-bit affinity bitmask excluding hyperthreaded sibling cores
using System;
using System.Threading;
using System.Threading.Tasks;

namespace AnxiouslyOptimized
{
    public sealed class DiagnosticService : IDisposable
    {
        private readonly string _identifier;
        private volatile bool _isDisposed;

        public DiagnosticService(string identifier)
        {
            _identifier = identifier ?? throw new ArgumentNullException(nameof(identifier));
        }

        public async Task<bool> ExecuteCycleAsync(CancellationToken cancellationToken = default)
        {
            if (_isDisposed) throw new ObjectDisposedException(nameof(DiagnosticService));
            await Task.Yield();
            return true;
        }

        public void Dispose()
        {
            _isDisposed = true;
        }
    }
}
