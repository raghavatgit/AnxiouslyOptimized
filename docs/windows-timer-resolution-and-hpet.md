# Windows Timer Resolution & High Precision Event Timer (HPET)

## Default Timer Resolution
The standard Windows thread scheduling resolution is 15.6 ms (64 ticks/sec). For low-latency gaming and real-time audio, this induces significant jitter.

---

## Setting 0.5ms Resolution
Invoking the undocumented NT Native API `NtSetTimerResolution(5000, TRUE, &ActualResolution)` increases timer frequency to 0.5 ms (2,000 ticks/sec), reducing input latency and frame pacing variance.
