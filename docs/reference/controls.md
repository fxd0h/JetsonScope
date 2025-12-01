## Controls Reference

- `jetson_clocks`: on/off/toggle (requires sudo/auth if set).
- `nvpmodel`: one of detected modes (validated).
- `fan`: 0–100 (%).
- `cpu_governor`: validated against `scaling_available_governors`.
- `gpu_governor`: validated against devfreq `available_governors` (e.g., `nvhost_podgov`, `performance`).
- `gpu_railgate`: `auto|on` (power/control).

Auth: `JETSONSCOPE_AUTH_TOKEN` (legacy `TEGRA_AUTH_TOKEN`) when set; otherwise open.

Exposure:
- `ListControls` via socket/CLI.
- `SetControl` via socket/CLI (jscopectl set …).
- Telemetry: control support flags and current states exported in Prometheus (`jetsonscope_control_*`).
