# JetsonScope

A modern, feature-rich Terminal User Interface (TUI) for monitoring NVIDIA Jetson devices, written in Rust using `ratatui`.

## Features

- **Real-time Monitoring**: CPU, GPU, RAM, SWAP, temperatures, and power consumption
- **Modern UI**: Animated gauges, color cycling, neon aesthetics
- **Client-Server Architecture**: Daemon (`jscoped`) + TUI client (`jscope`) + CLI (`jscopectl`)
- **Hardware Detection**: Automatic detection of Jetson model, L4T version, and capabilities
- **Control Management**: Fan speed, NVPModel modes, jetson_clocks
- **Process Monitoring**: Top processes by CPU/memory usage
- **Cross-Platform**: Works on macOS (emulator) and Jetson devices

## Installation

### Prerequisites
- Rust toolchain (recommended: stable via `rustup`):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustup default stable
  ```
- On Jetson (JetPack), ensure build essentials:
  ```bash
  sudo apt-get update
  sudo apt-get install -y build-essential pkg-config libssl-dev
  ```
  (You can install Rust the same way on Jetson; cross-compiling isn’t required if you build on-device.)

```bash
# Build all binaries
cargo build --release --all-features

# Or build specific binaries
cargo build --release --features daemon  # jscoped only
cargo build --release --features tui     # jscope only
cargo build --release --features cli     # jscopectl only

# Offline build (after vendoring)
make vendor
cargo build --offline --release --all-features
```

## Usage

### Build Locally

```bash
# Build all binaries (TUI + daemon + CLI)
cargo build --release

# Build specific binaries
cargo build --release --bin jscope      # TUI only
cargo build --release --bin jscoped     # Daemon only
cargo build --release --bin jscopectl   # CLI only

# Run without installing
cargo run --bin jscope                 # TUI
cargo run --bin jscoped                # Daemon
cargo run --bin jscopectl -- stats     # CLI
```

### Scripts (service/logging/control helpers)

```bash
# Manage daemon (uses systemd if available, else direct)
./scripts/jetsonscope-service.sh status
./scripts/jetsonscope-service.sh start

# Periodic stats logger (json|cbor)
./scripts/jetsonscope-logger.sh 1000 json

# List or set controls
./scripts/jetsonscope-control.sh list
./scripts/jetsonscope-control.sh set fan 70

# More in docs/scripts.md
```

### Docker

```bash
docker build -t jetsonscope .
# TUI (default, like original jtop)
docker run --rm -it jetsonscope
# Daemon
docker run --rm -it -e MODE=daemon -e JETSONSCOPE_SOCKET_PATH=/tmp/jetsonscope.sock jetsonscope
# From host (CLI)
docker exec -it jetsonscope jscopectl stats

# More in docs/docker.md
```

### Tests

```bash
# All tests (unit + integration + examples)
cargo test --all
# Test evidence: docs/tests.md
```

### Telemetry

- Enable periodic health logging (JSONL):
  ```bash
  export JETSONSCOPE_TELEMETRY_LOG=/tmp/jscoped-health.log
  export JETSONSCOPE_TELEMETRY_INTERVAL=30   # seconds, optional (default 30)
  jscoped
  ```
- Health via CLI: `jscopectl health`
- Prometheus-style metrics:
  ```bash
  JETSONSCOPE_HTTP_ADDR=0.0.0.0:9090 jscoped
  # Scrape http://<host>:9090/ for metrics
  # Optional: export JETSONSCOPE_METRICS_TOKEN and use Authorization: Bearer <token>
  ```
See `docs/telemetry.md` for details.

### Packaging for Jetson (.tar.gz)

```bash
# Generate tarball with binaries + systemd unit (default: current arch)
packaging/jetson/pack.sh
# Optional: force arch/profile
ARCH=aarch64 PROFILE=release packaging/jetson/pack.sh

# On the Jetson (as root)
sudo tar -C / -xzf jetsonscope-<version>-<arch>.tar.gz
sudo systemctl daemon-reload
sudo systemctl enable jscoped
sudo systemctl start jscoped
jscope
```

### Running the Daemon

```bash
# Start daemon manually
jscoped
# Or with cargo
cargo run --bin jscoped

# With systemd (after install.sh)
sudo systemctl start jscoped
sudo systemctl status jscoped
sudo systemctl enable jscoped  # Auto-start on boot
```

### TUI Client

```bash
# Start the TUI
jscope
# Or with cargo
cargo run --bin jscope

# Keybindings:
# q - Quit
# v - Cycle views (Dashboard → Processes → GPU Engines → Clocks/Governors)
# h - Toggle help panel
# s - Sort processes CPU/Mem (Processes view)
# r - Reconnect to socket
# c - Toggle jetson_clocks (requires daemon)
# m - Cycle nvpmodel mode (requires daemon)
# f - Set fan to 80% (demo, requires daemon)

Views:
- Dashboard: RAM/SWAP/IRAM, per-core CPU gauges, GPU load, engines table, temps, power rails.
- Processes: Top processes by CPU.
- GPU Engines: Gauges for all engines (GR3D, EMC, NVENC/DEC/JPG, VIC, OFA, ISP, NVCSI, APE).
- Clocks/Governors: CPU summary, EMC/MC/AXI clocks, GPU/media engines, control states.

Feature parity vs jtop (current snapshot):
- Metrics: RAM/SWAP/IRAM, per-core CPU load/freq, engines (EMC/MC/AXI/GR3D/NVENC/NVDEC/NVJPG/NVJPG1/VIC/OFA/ISP/NVCSI/APE/PCIE), temps, power rails, controls (jetson_clocks/nvpmodel/fan).
- TUI: Dashboard, Processes (CPU/Mem sort, UID/threads), GPU Engines grid, Clocks/Governors view.
- Telemetry: Prometheus/REST exports all parsed engines and control status; health JSONL logging.
- Controls: jetson_clocks, nvpmodel, fan setpoint, cpu_governor, gpu_governor, gpu_railgate (validated, auth token, safe no-op off-Jetson).
- Known gaps to close vs jtop:
  - Parity matrix + screenshots (see docs/parity.md; screenshots TODO).
  - ISP/NVCSI/NVLINK richer detail may require larger SKUs; currently exposed as generic engines when present.
  - Examples: Rust examples provided (stats, hardware, controls, telemetry, snapshot, debug snapshot, config presets, jetson_release/jetson_swap/env vars).
  - Snapshot/logging helper: use `jscopectl stats --json > file` (script optional).
```

### CLI Client

```bash
# Get current stats
jscopectl stats
# Or with cargo
cargo run --bin jscopectl -- stats

# Get hardware metadata
jscopectl meta

# List available controls
jscopectl list

# Set fan speed (0-100%)
jscopectl set fan 75

# Set NVPModel mode
jscopectl set nvpmodel MAXN

# Toggle jetson_clocks
jscopectl set jetson_clocks on

# Daemon health/telemetry
jscopectl health

# Helper utilities (installed as binaries):
# jetson_scope_release    - prints meta (model/soc/l4t/jetpack/cuda arch)
# jetson_scope_swap       - prints swap usage
# jetson_scope_env        - prints JetsonScope/Tegra env vars
# jetson_scope_snapshot   - writes snapshot.json (stats)
# jetson_scope_config     - list/set controls or apply presets
```

### Examples (like jetson_stats samples)

```bash
# Quick snapshot
cargo run --example stats
# Hardware info
cargo run --example hardware
# List controls or set fan/nvpmodel/jetson_clocks
cargo run --example controls          # list
cargo run --example controls -- set fan 60
```

### Environment Variables

```bash
# Custom socket path (default: /tmp/jetsonscope.sock, legacy: /tmp/tegrastats.sock)
export JETSONSCOPE_SOCKET_PATH=/custom/path/jetsonscope.sock   # fallback: TEGRA_SOCKET_PATH

# Protocol selection (default: json, options: json, cbor)
export JETSONSCOPE_PROTO=cbor   # fallback: TEGRA_PROTO

# Authentication token for control actions
export JETSONSCOPE_AUTH_TOKEN=my-secret-token   # fallback: TEGRA_AUTH_TOKEN

# Force TUI mode (for development/testing)
export JETSONSCOPE_TUI_MODE=emulator  # or synthetic (fallback: TEGRA_TUI_MODE)

# Custom tegrastats command (for emulation)
export JETSONSCOPE_STATS_CMD="python3 ../tegrastats_emulator.py --interval 1000"  # fallback: TEGRASTATS_CMD
```

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_parse_ram
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Check without building
cargo check
```

### Makefile

```bash
# Run all CI checks
make ci

# Individual commands
make fmt
make clippy
make test
make build
```

## Architecture

```
┌─────────────┐
│   jscope    │  TUI Client
│   (Rust)    │
└──────┬──────┘
       │
       │ UNIX Socket
       │ /tmp/jetsonscope.sock
       │
┌──────▼──────┐
│  jscoped   │  Daemon
│   (Rust)    │
└──────┬──────┘
       │
       ├─► tegrastats (real)
       ├─► emulator.py (dev)
       └─► synthetic (fallback)
```

## Project Structure

```
JetsonScope/
├── src/
│   ├── main.rs           # TUI entry point
│   ├── app.rs            # Application state
│   ├── ui.rs             # Rendering logic
│   ├── parser.rs         # Tegrastats parser
│   ├── collector.rs      # Data collection
│   ├── control.rs        # Hardware controls
│   ├── hardware.rs       # Hardware detection
│   ├── protocol.rs       # Client-server protocol
│   ├── processes.rs      # Process monitoring
│   └── bin/
│       ├── jscoped.rs        # Daemon
│       └── jscopectl.rs      # CLI client
├── install/
│   └── jscoped.service       # Systemd unit
├── install.sh            # Installation script
└── Cargo.toml
```

## Troubleshooting

### Socket Connection Failed

```bash
# Check if daemon is running
sudo systemctl status jscoped

# Check socket exists
ls -l /tmp/jetsonscope.sock

# Restart daemon
sudo systemctl restart jscoped
```

### Permission Denied

Controls (fan, nvpmodel, jetson_clocks) require root privileges:

```bash
# Run daemon as root
sudo jscoped

# Or use systemd (already runs as root)
sudo systemctl start jscoped
```

### Emulator Mode (Development)

```bash
# Force emulator mode
export JETSONSCOPE_TUI_MODE=emulator
cargo run

# Or specify custom command
export JETSONSCOPE_STATS_CMD="python3 ../tegrastats_emulator.py --interval 1000"
cargo run
```

## Protocol Documentation

### Communication Format

The daemon and clients communicate via UNIX socket (`/tmp/jetsonscope.sock`) using JSON or CBOR serialization (auto-detected).

### Request Types

```rust
enum Request {
    GetStats,           // Get current stats snapshot
    GetMeta,            // Get hardware metadata
    ListControls,       // List available controls
    SetControl {        // Set a control value
        control: String,  // Control name: "fan", "nvpmodel", "jetson_clocks"
        value: String,    // New value: "80", "MAXN", "on"
        token: Option<String>  // Optional auth token (JETSONSCOPE_AUTH_TOKEN / TEGRA_AUTH_TOKEN)
    }
}
```

### Response Types

```rust
enum Response {
    Stats {
        source: String,           // Data source: "socket", "command", "synthetic"
        data: Option<TegraStats>  // Parsed stats or None
    },
    Meta(JetsonHardware),        // Hardware info
    Controls(Vec<ControlInfo>),  // List of controls
    ControlState(ControlInfo),   // Updated control state after SetControl
    Error(ErrorInfo)             // Structured error
}
```

### Control Information

```rust
struct ControlInfo {
    name: String,              // "fan", "nvpmodel", "jetson_clocks"
    description: String,       // Human-readable description
    value: String,             // Current value
    options: Vec<String>,      // Available options
    readonly: bool,            // Whether control is read-only
    min: Option<u32>,          // Minimum value (numeric controls)
    max: Option<u32>,          // Maximum value (numeric controls)
    step: Option<u32>,         // Step size
    requires_sudo: bool,       // Whether control requires root
    supported: bool,           // Whether supported on this hardware
    unit: Option<String>       // Unit: "%", "MHz", etc.
}
```

### Error Codes

```rust
struct ErrorInfo {
    code: String,    // Error code
    message: String  // Human-readable message
}
```

**Common error codes:**
- `auth_failed`: Authentication failed (invalid or missing token)
- `invalid_control`: Unknown control name
- `control_error`: Control operation failed (validation, execution)
- `lock_error`: Internal lock error

### Authentication

Optional authentication via `JETSONSCOPE_AUTH_TOKEN` environment variable (fallback: `TEGRA_AUTH_TOKEN`):

```bash
# On daemon
export JETSONSCOPE_AUTH_TOKEN="my-secret-token"
jscoped

# On client
export JETSONSCOPE_AUTH_TOKEN="my-secret-token"
jscopectl set fan 80
```

If `JETSONSCOPE_AUTH_TOKEN` (or legacy `TEGRA_AUTH_TOKEN`) is set on daemon, all `SetControl` requests must include a matching token.

### Example Requests/Responses

**GetStats:**
```json
Request: {"GetStats": null}
Response: {
  "Stats": {
    "source": "socket",
    "data": { "timestamp": "...", "ram": {...}, ... }
  }
}
```

**ListControls:**
```json
Request: {"ListControls": null}
Response: {
  "Controls": [
    {
      "name": "fan",
      "value": "50",
      "min": 0,
      "max": 100,
      "unit": "%",
      "supported": true,
      "requires_sudo": true,
      ...
    }
  ]
}
```

**SetControl (success):**
```json
Request: {
  "SetControl": {
    "control": "fan",
    "value": "75",
    "token": "my-secret-token"
  }
}
Response: {
  "ControlState": {
    "name": "fan",
    "value": "75",
    ...
  }
}
```

**SetControl (error):**
```json
Response: {
  "Error": {
    "code": "control_error",
    "message": "Fan value must be 0-100"
  }
}
```

## License

MIT

## Contributing

Contributions welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Run `make ci` to ensure tests pass
4. Submit a pull request
