## JetsonScope Docs

This directory mirrors the structure of the original jetson_stats docs, adapted for JetsonScope.

- `scripts.md`: using helper scripts (service, control, logger).
- `docker.md`: building and running JetsonScope in Docker.
- `telemetry.md`: health logging and Prometheus metrics.
- `tests.md`: test evidence and how to run them.
- `parity.md`: parity matrix vs jetson_stats/jtop + screenshots TODO.
- `reference/`: protocol, metrics, controls reference + tools index.
- Parity snapshot:
  - Metrics: RAM/SWAP/IRAM, CPU load/freq, engines (EMC/MC/AXI/GR3D/NVENC/NVDEC/NVJPG/NVJPG1/VIC/OFA/ISP/NVCSI/APE/PCIE), temps, power rails, controls (jetson_clocks/nvpmodel/fan).
  - TUI: Dashboard, Processes (CPU/Mem with sort, UID/threads), GPU Engines grid, Clocks/Governors view.
  - Telemetry: Prometheus/REST exports all parsed engines plus control status (jetson_clocks, nvpmodel, fan, cpu_governor, gpu_governor, gpu_railgate); health JSONL logging.
  - TODO: richer ISP/NVCSI/NVLINK detail if larger-SKU samples appear (currently exposed as generic engines when present); parity matrix/screenshots.
  - Known gaps vs original `jetson_stats` (to close when possible):
    - Parity matrix/screenshots: add visual comparison of panels/fields and sample outputs.
    - ISP/NVCSI/NVLINK detail: parsed generically; richer data may need larger SKUs or placeholders documented.
    - Governors toggle: jtop sometimes exposes CPU governor switches; decide to implement or document not provided.
    - Examples folder: add Rust examples mirroring jtop examples/ for protocol/telemetry usage.
    - Snapshot/logging helper: document `jscopectl stats --json > file` (or add a helper script) to mirror jtop logging.

See the root README for quickstart; this folder contains detailed references.
