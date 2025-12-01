## Jetson Packaging (.tar.gz)

This flow produces a tarball ready to deploy on a Jetson (aarch64) with binaries, the systemd unit, and a quick README.

### Requirements
- Rust stable toolchain and `cargo`
- Write access to the project tree (`target/` is used)
- On Jetson you build natively; on x86_64 you need a cross toolchain to produce Jetson-native binaries.

### How to generate the package
```bash
# From repo root
packaging/jetson/pack.sh

# Optional parameters
ARCH=aarch64 PROFILE=release packaging/jetson/pack.sh
```

The script creates:
- Binaries in `target/package/.../usr/local/bin` (`jscope`, `jscoped`, `jscopectl`)
- Systemd unit in `target/package/.../etc/systemd/system/jscoped.service`
- Final tarball at `target/jetsonscope-<version>-<arch>.tar.gz`

### Installing the tarball on Jetson
```bash
# Copy the tarball to the Jetson and as root:
sudo tar -C / -xzf jetsonscope-<version>-<arch>.tar.gz
sudo systemctl daemon-reload
sudo systemctl enable jscoped
sudo systemctl start jscoped
# Then launch the TUI
jscope
```

### Notes
- The tarball assumes `/usr/local/bin` and `/etc/systemd/system`.
- For offline builds, run `make vendor` first and then use `cargo build --offline` inside the script if needed.
