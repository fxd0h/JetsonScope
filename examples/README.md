# JetsonScope Examples

These examples mirror the use cases from the original `jetson_stats` examples, using JetsonScope's JSON/CBOR socket protocol.

Common flags (env):
- `JETSONSCOPE_SOCKET_PATH` (fallback `TEGRA_SOCKET_PATH`) default `/tmp/jetsonscope.sock` (legacy `/tmp/tegrastats.sock`)
- `JETSONSCOPE_PROTO` (fallback `TEGRA_PROTO`) set to `cbor` to use CBOR
- `JETSONSCOPE_AUTH_TOKEN` (fallback `TEGRA_AUTH_TOKEN`) when controls require auth

Run:
```bash
cargo run --example stats
cargo run --example hardware
cargo run --example controls          # list controls
cargo run --example controls -- set fan 60   # set control
# CLI binaries are named jscope (TUI) and jscopectl (CLI) if you want to run directly after build.

Included examples:
- `stats.rs`: fetch current stats snapshot (JSON/CBOR)
- `hardware.rs`: fetch meta information
- `controls.rs`: list controls and optionally set one (jetson_clocks/nvpmodel/fan/cpu_governor/gpu_governor/gpu_railgate)
- `telemetry.rs`: scrape the Prometheus metrics endpoint (requires `JETSONSCOPE_HTTP_ADDR`, optional `JETSONSCOPE_METRICS_TOKEN`)
- `snapshot.rs`: write a single stats snapshot to `snapshot.json` (like jtop logging/snapshot use-case)
- `debug_snapshot.rs`: fetch `/debug/snapshot` over HTTP (requires `JETSONSCOPE_HTTP_ADDR`, optional `JETSONSCOPE_DEBUG_TOKEN`)
- `config.rs`: simple presets (performance/balanced) using controls (jetson_clocks, cpu_governor, etc.)
- `jetson_release.rs`: print meta info (model/SoC/Jetpack/L4T/CUDA)
- `jetson_swap.rs`: print SWAP usage from stats
- `env_vars.rs`: list JetsonScope/legacy Tegra environment variables
- `jetson_scope_config` binary: installable helper (preset/set controls) via Cargo bin targets
```
