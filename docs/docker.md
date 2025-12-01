# Docker Usage

JetsonScope provides a Dockerfile to build and run the daemon/TUI stack similarly to the original project.

## Build
```bash
# From tegrastats_tui/
docker build -t jetsonscope .
```

## Run (TUI by default, like original jtop)
```bash
docker run --rm -it jetsonscope         # runs jscope (TUI)
```

## Run daemon
```bash
docker run --rm -it \
  --name jetsonscope \
  -e JETSONSCOPE_SOCKET_PATH=/tmp/jetsonscope.sock \
  -e MODE=daemon \
  jetsonscope
```

Notes:
- Default entrypoint runs the TUI; set `MODE=daemon` to run the daemon, `MODE=cli` to run `jscopectl`.
- For host socket sharing, mount a directory and set `JETSONSCOPE_SOCKET_PATH` accordingly.
- To use the CLI from host, you can `docker exec -it jetsonscope jscopectl stats`.
- If you need the TUI inside the container, run with a TTY and `docker exec -it jetsonscope jscope`.
- Telemetry inside container:
  - Health log: set `JETSONSCOPE_TELEMETRY_LOG=/tmp/jscoped-health.log`
  - Prometheus metrics: set `JETSONSCOPE_HTTP_ADDR=0.0.0.0:9090` and expose the port
