# Test Evidence

Commands executed (macOS host):
- `cargo test --all` (passes)
- Included suites:
  - Unit tests (control, parser) in `src/`
  - Parser edge tests in `tests/parser_samples.rs`
  - Control mock tests in `tests/control_mocks.rs`
  - Socket integration tests in `tests/integration_socket.rs` (pass when socket available; on this host they passed)
  - Telemetry logging exercised via `jscoped` env: set `JETSONSCOPE_TELEMETRY_LOG` to log JSONL health snapshots
- Prometheus/HTTP metrics: `JETSONSCOPE_HTTP_ADDR=0.0.0.0:9090 jscoped` (serves Prometheus text metrics)

Notes:
- Mocked control tests avoid real Jetson commands via `ControlManager::mock`.
- Parser tests cover Orin/Xavier/Nano-style samples, negative temps, power/engines.
