#!/bin/bash
# Install script for JetsonScope on Jetson devices

set -e

echo "Installing JetsonScope..."

# Check if running on Jetson
if [ ! -f /etc/nv_tegra_release ]; then
    echo "Warning: Not running on a Jetson device. Installation may not work correctly."
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Build binaries
echo "📦 Building release binaries..."
cargo build --release --locked --all-features

# Install binaries
echo "📥 Installing binaries to /usr/local/bin..."
sudo cp target/release/jscope /usr/local/bin/jscope
sudo cp target/release/jscoped /usr/local/bin/jscoped
sudo cp target/release/jscopectl /usr/local/bin/jscopectl

sudo chmod +x /usr/local/bin/jscope
sudo chmod +x /usr/local/bin/jscoped
sudo chmod +x /usr/local/bin/jscopectl

echo "✅ Binaries installed:"
echo "   - jscope (TUI client)"
echo "   - jscoped (daemon)"
echo "   - jscopectl (CLI client)"
echo ""

# Install systemd service
if [ -d /etc/systemd/system ]; then
    echo "📋 Installing systemd service..."
    sudo cp install/jscoped.service /etc/systemd/system/
    sudo systemctl daemon-reload
    sudo systemctl enable jscoped
    sudo systemctl start jscoped
    
    echo "✅ Systemd service installed and started"
    echo "   Socket: /tmp/jetsonscope.sock"
    echo ""
    echo "   Manage with:"
    echo "   - sudo systemctl status jscoped"
    echo "   - sudo systemctl restart jscoped"
    echo "   - sudo systemctl stop jscoped"
else
    echo "⚠️  Systemd not found, skipping service installation"
    echo "   Run daemon manually: jscoped"
fi

echo ""
echo "🎉 Installation complete!"
echo ""
echo "Usage:"
echo "  jscope              # Start TUI"
echo "  jscopectl stats     # Get stats via CLI"
echo "  jscopectl list      # List controls"
echo ""
echo "Environment variables:"
  echo "  JETSONSCOPE_SOCKET_PATH=/custom/path.sock (fallback: TEGRA_SOCKET_PATH)"
  echo "  JETSONSCOPE_PROTO=cbor    (fallback: TEGRA_PROTO)"
  echo "  JETSONSCOPE_AUTH_TOKEN=secret (fallback: TEGRA_AUTH_TOKEN)"
