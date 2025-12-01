# Protocol Reference

- Socket: default `/tmp/jetsonscope.sock` (legacy `/tmp/tegrastats.sock`); override with `JETSONSCOPE_SOCKET_PATH`/`TEGRA_SOCKET_PATH`.
- Encoding: JSON by default; CBOR if payload is CBOR and `JETSONSCOPE_PROTO=cbor`/`TEGRA_PROTO=cbor`.

## Requests
- `GetStats` → `Response::Stats { source, data: Option<TegraStats> }`
- `GetMeta` → `Response::Meta(JetsonHardware)`
- `ListControls` → `Response::Controls(Vec<ControlInfo>)`
- `SetControl { control, value, token }` → `Response::ControlState(ControlInfo)` or `Response::Error`

## Responses
- `Stats`: latest tegrastats snapshot plus source label.
- `Meta`: hardware detection (model, SoC, L4T/JetPack, engines, rails, governors, nvpmodel modes).
- `Controls`: control capabilities (name, options, sudo flag, supported, unit, min/max/step).
- `Error`: `ErrorInfo { code, message }`.
- `Health` (via CLI): daemon health counters.

## Controls (names/values)
- `jetson_clocks`: `on|off|toggle`
- `nvpmodel`: one of detected modes (e.g., `MAXN`, `15W`, etc.)
- `fan`: `0-100` (%)
- `cpu_governor`: detected from `scaling_available_governors` (e.g., `ondemand`, `performance`)
- `gpu_governor`: detected from devfreq `available_governors` (e.g., `nvhost_podgov`, `performance`)
- `gpu_railgate`: `auto|on`
- Auth: `JETSONSCOPE_AUTH_TOKEN` (legacy `TEGRA_AUTH_TOKEN`) required if set; otherwise open.

## Telemetry/HTTP
- `JETSONSCOPE_HTTP_ADDR=host:port` enables HTTP server (`/metrics`, `/debug/snapshot`, `/debug/processes`).
- Auth: `JETSONSCOPE_METRICS_TOKEN`, `JETSONSCOPE_DEBUG_TOKEN` (Bearer).
- Health log: `JETSONSCOPE_TELEMETRY_LOG`, interval `JETSONSCOPE_TELEMETRY_INTERVAL` (s).

See also: `docs/telemetry.md` for metric names and `examples/controls.rs` for usage.
