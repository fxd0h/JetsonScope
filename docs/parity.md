# Parity Matrix (JetsonScope vs jetson_stats/jtop)

| Area | jtop | JetsonScope |
| --- | --- | --- |
| Metrics: Memory | RAM/SWAP/IRAM + LFB | RAM/SWAP/IRAM + LFB (demo placeholders) |
| Metrics: CPU | Per-core load/freq, governors (read) | Per-core load/freq, CPU governor (read/write) |
| Metrics: Engines | GR3D, EMC/MC/AXI, NVENC/NVDEC/NVJPG/VIC/OFA/ISP/NVCSI/PCIE/NVLINK/APE | Same set parsed; UTIL tokens handled; placeholders if absent |
| Metrics: Temps | All sensors from tegrastats | Same |
| Metrics: Power | Rails current/avg | Same |
| Controls | jetson_clocks, nvpmodel, fan; GPU scaling/railgate on some SKUs | jetson_clocks, nvpmodel, fan, cpu_governor, gpu_governor, gpu_railgate (validated, auth token, safe no-op off-Jetson) |
| TUI views | Dashboard, Processes, GPU/Clocks panels | Dashboard, Processes (CPU/Mem sort, UID/threads), GPU Engines grid, Clocks/Governors |
| Telemetry | N/A (jtop is local) | Prometheus/REST + health JSONL; control status exported |
| Scripts/Docker | Helpers + Docker | Helper scripts + Docker (build/runtime) |
| Examples | Python examples | Rust examples (stats, hardware, controls, telemetry) |
| Missing/Notes | ISP/NVCSI/NVLINK detail depends on SKU; logging/export helpers | Same; snapshot via `jscopectl stats --json > file`; refine ISP/NVCSI/NVLINK with larger-SKU samples |

# Screenshots (placeholders)
- Dashboard: `images/dashboard.png` (RAM/SWAP/IRAM, CPU gauges, GPU, engines table, temps/power).
- Processes: `images/processes.png` (CPU/Mem sort, UID/threads).
- GPU Engines: `images/gpu_engines.png` (GR3D/EMC/NVENC/DEC/JPG/VIC/OFA/ISP/NVCSI/NVLINK/PCIE/APE gauges).
- Clocks/Governors: `images/clocks.png` (CPU summary + governor, EMC/MC/AXI, GPU/media engines, controls state).

> TODO: capture real screenshots (demo mode acceptable) and place under `docs/images/`.

# Controls summary
- jetson_clocks (on/off/toggle)
- nvpmodel (validated modes)
- fan (0–100%)
- cpu_governor (validated against available governors)
- gpu_governor (validated against available governors via devfreq)
- gpu_railgate (auto/on where supported)

# Telemetry endpoints
- `/metrics` (Prometheus): engines, memory, temps, power, control states.
- `/debug/snapshot`: stats + control status JSON.
- Health JSONL: `JETSONSCOPE_TELEMETRY_LOG`.
