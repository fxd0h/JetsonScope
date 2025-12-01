# Roadmap: JetsonScope (jetson_stats-inspired, Rust + ratatui)
# Note: Mark tasks as done only when documented and tested (state doc/tests when checking items).

Phase 0 · Current Base
- [x] Robust tegrastats parser (RAM/SWAP/IRAM/CPU/GPU/engines/temps/power)
- [x] Configurable data source (real tegrastats, Python emulator, synthetic) with fallback
- [x] Basic TUI with gauges, tables, trends (RAM/GPU/CPU) + controls panel
- [x] Minimal controls (jetson_clocks toggle, nvpmodel cycle, fan placeholder)

Phase 1 · Service/Daemon and API
- [x] Design minimal local protocol: JSON snapshot `{source, stats}` via UNIX socket `/tmp/jetsonscope.sock` (legacy `/tmp/tegrastats.sock`)
- [x] Shared collector:
  - [x] Auto-detect source (tegrastats/emulator/synthetic) and socket support (`JETSONSCOPE_SOCKET_PATH` / `TEGRA_SOCKET_PATH`)
  - [x] Cache last state and periodic publish (daemon returns snapshot on connect)
- [x] Daemon `jetsonscoped` exposes `/tmp/jetsonscope.sock`
- [x] Simple CLI client (`jetsonscopectl`) reads snapshot from socket
- [x] Collector prioritizes socket (`JETSONSCOPE_SOCKET_PATH`/`TEGRA_SOCKET_PATH` or `/tmp/jetsonscope.sock`) before command/emulator
- [x] Extend protocol: `get_meta` and `set_control` basics (JSON)
- [x] Expose control support/capabilities (jetson_clocks/nvpmodel/fan) in `ControlStatus`
- [x] Extend protocol: `list_controls` with detailed capabilities (JSON/CBOR)
- [x] TUI client always via socket (SocketOnly, visible reconnect/fallback, 'r' key)
- [x] Permission/error handling (clear messages, optional read-only/auth) with `ErrorInfo { code, message }`

Phase 2 · Hardware Detection and Capabilities (dynamic model)
- [x] Detect basic HW: is_jetson, nv_tegra_release, hostname (`get_meta`)
- [x] Port full Jetson detection: `/etc/nv_tegra_release`, device-tree, `tegrastats --verbose` for full map
- [x] Map sensors/rails/engines per SKU (no fixed hardcode)
- [x] Discover nvpmodel modes dynamically
- [x] Detect fan control support and CPU/GPU governors

Phase 3 · Robust Controls
- [x] Encapsulate jetson_clocks, nvpmodel, fan in safe module:
  - [x] Validate modes/ranges before executing
  - [x] Return detailed states and friendly errors
  - [x] Safe no-op on non-Jetson and when lacking permissions
- [x] Expose controls via socket/API with basic auth (optional token)
- [x] Add actions: set fan %, set specific nvpmodel, on/off jetson_clocks, maybe DVFS governors
- [x] Document and test each action (Jetson host + demo mode)

Phase 4 · Decoupled TUI and Advanced Views
- [x] TUI reads from socket (socket-only, visible reconnect/fallback, 'r' key)
- [x] Additional views:
  - [x] Top processes
  - [x] GPU engines grid
  - [x] Longer history graphs (sparklines up to 24h)
  - [x] Key help panel
- [x] Reconnection handling and fallback to synthetic if service goes down

Phase 5 · Packaging and DX
- [x] Cargo features: `daemon`, `tui`, `emulator` for separate builds (default tui+daemon)
- [x] Separate binaries: `jscoped` (daemon), `jscope` (TUI), `jscopectl` (CLI)
- [x] Scripts/systemd unit to install the service on Jetson
- [x] Usage/controls docs (README/TASKS updates)
- [x] Jetson packaging (.deb/.rpm or tarball) with service and final paths
- [x] Offline packaging/installer (vendor optional)

Phase 6 · Quality and Tests
- [x] Parser tests with real/edge samples (multi SKU)
- [x] Control tests (mocks for jetson_clocks/nvpmodel/jetson_fan commands)
- [x] Socket integration tests (request/response, reconnection)
- [x] Local CI workflow (fmt, clippy, tests) via GitHub Actions
- [x] Document test evidence per task (see docs/tests.md)

Optional / Improvement
- [x] Export Prometheus/REST for scraping
- [ ] Lightweight web dashboard sharing the same daemon
- [x] Daemon health/error telemetry
- [x] Parity with original project scripts/Docker/docs:
  - [x] Inventory original scripts and requirements
  - [x] Create equivalent scripts (service helpers, logging/snapshot, control helpers)
  - [x] Dockerfile and/or compose equivalent to the original
  - [x] Docs similar to the original `docs/` (install, usage, API, Docker, scripts)
- [x] Work plan for scripts/Docker/docs parity:
  - [x] Analyze original assets (scripts, Dockerfile/compose, docs) and enumerate required features/env
  - [x] Implement scripts parity (service helpers, snapshot/logging, control helper) with `JETSONSCOPE_*` env + legacy fallbacks
  - [x] Add Docker parity (build/runtime image, optional compose) matching original behavior
  - [x] Add docs mirroring original topics (install, usage, API/protocol, Docker, scripts) under `docs/`
  - [x] Wire Makefile targets (docker-build/run, scripts-check) and validate builds/examples/tests

Phase 7 · jtop Information Parity (plan — mark done only with docs/tests)
- [x] Inventory jtop UI/metrics: enumerate all panels/fields (CPU/EMC clocks, GPU/ISP/NVCSI, power rails, temps, RAM/SWAP/IRAM, processes table fields/sorting) and map to JetsonScope equivalents. (Mapping: CPU load/freq → Dashboard/Clocks; EMC/MC/AXI → engines + Clocks; GR3D/GPU engines incl. NVENC/NVDEC/NVJPG/VIC/OFA/ISP/NVCSI/NVLINK/PCIE/APE → engines view; RAM/SWAP/IRAM → Dashboard; temps/power rails → Dashboard; processes → CPU/Mem sort + UID/threads; controls → jetson_clocks/nvpmodel/fan/cpu_governor/gpu_governor/gpu_railgate; telemetry exports all parsed engines/controls.)
- [ ] Capture missing metrics from tegrastats --verbose/device-tree: EMC/MC/AXI clocks, NVCSI/ISP utilization, NVENC/NVDEC detail, PCIE/NVLINK counters if available; document source per metric. (Current Nano/Orin sample only exposed EMC/GR3D/NVDEC/NVJPG/VIC/OFA/APE; other fields likely require larger SKUs—fallback to jtop reference defaults if no samples.)
- [x] Source mapping (to collect samples and add parser tests):
  - EMC/MC/AXI clocks → `EMC_FREQ`, `EMC_UTIL`, `EMC_AVG`, `MC_FREQ` fields in `tegrastats --verbose`.
  - ISP/NVCSI → `ISP_UTIL`, `NVCSI_UTIL`, `NVCSI_ERR` lines (verbose).
  - NVENC/NVDEC detail → per-engine `NVENC`, `NVDEC` with freq/usage fields; add raw capture if variants differ.
  - PCIE/NVLINK (if present) → `PCIE_UTIL`, `NVLINK` counters in verbose output.
  - Governors/clocks → `GR3D_FREQ`, `CPU` cluster clocks, `EMC_FREQ`; correlate with device-tree for limits.
  - Processes enrichment → `ps`/`nvidia-smi` analog not available; reuse `sysinfo` to add MEM/threads/user columns and sorting.
 - [x] Parser extensions: handle bracketed freqs (`0%@[305]`), “off” engines, and retain EMC usage/freq; tests added with Nano/Orin verbose sample. (Further extensions for NVCSI/ISP/NVENC/NVDEC detail/PCIE/NVLINK blocked until samples available.)
 - [x] TUI parity: add panels/columns to match jtop info (clocks/governors view, NVCSI/ISP utilization, per-engine detail, richer process table with sorting/columns); gated by data availability and demo mode. (Added Clocks/Governors view, engine summaries, process table with CPU/Mem sort, UID, threads.)
 - [x] Telemetry parity: expose all new metrics via Prometheus/REST and include in `/debug/snapshot`; document endpoints.
 - [x] Controls parity review: confirm jtop control set (jetson_clocks, nvpmodel, fan) covered; if jtop exposes governor toggles or logging/export toggles, note decision and implement or document gap. (Implemented cpu_governor, gpu_governor, gpu_railgate; logging helper documented via jscopectl stats --json.)
 - [ ] Documentation/tests: update README/docs to show new panels/metrics, add parser/TUI tests, and note coverage status against jtop. (Parity matrix added, screenshots pending.)
- [ ] Parity tracking (current vs missing) — update as tasks complete:
 - Current: RAM/SWAP/IRAM, per-core CPU load/freq, engines (GPU/others) usage/freq/raw, temps, power rails, fan/nvpmodel/jetson_clocks controls, processes (CPU/Mem sort, UID/threads), telemetry/Prometheus, Clocks/Governors view.
 - Missing (to implement): ISP/NVCSI utilization (if available), detailed NVENC/NVDEC fields per SKU, PCIE/NVLINK counters (if available), export/logging toggle parity, documentation parity matrix/screenshots.
- [ ] Immediate next steps:
  - Add parity matrix + screenshots in README/docs showing new panels/metrics. (Matrix in docs/parity.md; screenshots pending.)
  - If larger-SKU samples arrive, extend parser for ISP/NVCSI/NVLINK specifics; else document placeholders/defaults.
  - Add examples folder (Rust snippets for protocol/telemetry) to mirror jtop examples. (Done: stats, hardware, controls, telemetry, snapshot, debug snapshot, config presets, jetson_release/jetson_swap/env vars).
