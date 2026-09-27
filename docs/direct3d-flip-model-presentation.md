# Direct3D DXGI Flip Model & DWM Independent Flip

## Legacy Blt vs Flip Model
* **BitBlt Model:** The application copies backbuffers into DWM surfaces, adding 1 frame of presentation latency.
* **Flip Model (`DXGI_SWAP_EFFECT_FLIP_DISCARD`):** The application queues buffers directly to the display hardware scanout engine, bypassing DWM composition without requiring exclusive fullscreen.
