# JetsonScope Scripts

Helper scripts (aligned with original jetson_stats tooling):

- `scripts/jetsonscope-service.sh`  
  Manage daemon via systemd if available, or run in foreground.  
  Usage: `./scripts/jetsonscope-service.sh [start|stop|restart|status|foreground]`

- `scripts/jetsonscope-logger.sh`  
  Periodically fetch stats (JSON/CBOR) for logging.  
  Usage: `./scripts/jetsonscope-logger.sh [interval_ms] [json|cbor]`

- `scripts/jetsonscope-control.sh`  
  List or set controls.  
  Usage: `./scripts/jetsonscope-control.sh list`  
  `./scripts/jetsonscope-control.sh set fan 70`

- Health/telemetry via CLI: `jscopectl health`

Environment (all scripts honor both JetsonScope and legacy vars):
- `JETSONSCOPE_SOCKET_PATH` (fallback `TEGRA_SOCKET_PATH`, default `/tmp/jetsonscope.sock`, legacy `/tmp/tegrastats.sock`)
- `JETSONSCOPE_PROTO` (`json|cbor`, fallback `TEGRA_PROTO`)
- `JETSONSCOPE_AUTH_TOKEN` (fallback `TEGRA_AUTH_TOKEN`)
- `JETSONSCOPE_TELEMETRY_LOG` / `JETSONSCOPE_TELEMETRY_INTERVAL` for health logging
- `JETSONSCOPE_HTTP_ADDR` for Prometheus metrics (daemon)
