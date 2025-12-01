# Telemetry

JetsonScope provides two telemetry outputs:

1) Health logging (JSONL)
```
export JETSONSCOPE_TELEMETRY_LOG=/tmp/jscoped-health.log
export JETSONSCOPE_TELEMETRY_INTERVAL=30   # seconds (optional, default 30)
jscoped
```
Each interval, a JSON health snapshot is appended (uptime, requests, errors, stats collected, connected clients).

2) Prometheus text metrics (HTTP) — now includes extended engines for jtop parity
```
JETSONSCOPE_HTTP_ADDR=0.0.0.0:9090 jscoped
# Scrape http://<host>:9090/ for metrics
# Optional auth:
export JETSONSCOPE_METRICS_TOKEN=secret
# Then use: curl -H "Authorization: Bearer secret" http://<host>:9090/
```
Exposed metrics:
- Health:
  - `jetsonscope_uptime_seconds` (gauge)
  - `jetsonscope_requests_total` (counter)
  - `jetsonscope_errors_total` (counter)
  - `jetsonscope_stats_collected_total` (counter)
  - `jetsonscope_connected_clients` (gauge)
- System snapshot (latest stats):
  - RAM/SWAP:
    - `jetsonscope_ram_bytes_total` (gauge)
    - `jetsonscope_ram_bytes_used` (gauge)
    - `jetsonscope_swap_bytes_total` (gauge)
    - `jetsonscope_swap_bytes_used` (gauge)
  - CPU per core:
    - `jetsonscope_cpu_core_load_percent{core="<idx>"}` (gauge)
    - `jetsonscope_cpu_core_freq_mhz{core="<idx>"}` (gauge)
  - Engines (e.g., GR3D, EMC, NVENC/NVDEC, etc.):
    - `jetsonscope_engine_usage_percent{engine="<name>"}` (gauge)
    - `jetsonscope_engine_freq_mhz{engine="<name>"}` (gauge)
  - Temperatures:
    - `jetsonscope_temp_celsius{sensor="<name>"}` (gauge)
  - Power rails:
    - `jetsonscope_power_mw_current{rail="<name>"}` (gauge)
    - `jetsonscope_power_mw_average{rail="<name>"}` (gauge)
- Control status:
  - `jetsonscope_control_supported{control="fan"|...}` (gauge 0/1)
  - `jetsonscope_control_jetson_clocks_on` (gauge 0/1)
  - `jetsonscope_control_fan_percent` (gauge)
  - `jetsonscope_control_nvpmodel_mode{mode="<name>"}` (info gauge)
  - `jetsonscope_control_nvpmodel_supported_modes{mode="<name>"}` (info gauge)
  - `jetsonscope_control_cpu_governor{governor="<name>"}` (info gauge)
  - `jetsonscope_control_gpu_governor{governor="<name>"}` (info gauge)
  - `jetsonscope_control_gpu_railgate` (1=auto, 0=on)
  - `jetsonscope_control_last_error{message="<msg>"}` (info gauge)
- Engines/Clocks (new jtop-parity metrics):
  - `jetsonscope_engine_usage_percent{engine="EMC|GR3D|MC|AXI|NVENC|NVDEC|NVJPG|NVJPG1|VIC|OFA|ISP|NVCSI|PCIE"}` (gauge)
  - `jetsonscope_engine_freq_mhz{engine="..."}`
  - `jetsonscope_engine_raw_value{engine="APE"}` (when frequency-only)

Example curl:
```
curl -H "Authorization: Bearer $JETSONSCOPE_METRICS_TOKEN" http://localhost:9090/
```

Notes:
- Metrics are served in Prometheus text format.
- Health also available via CLI: `jscopectl health`.
- For tests run, see `docs/tests.md`.
