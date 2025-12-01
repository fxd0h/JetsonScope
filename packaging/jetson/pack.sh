#!/usr/bin/env bash
set -euo pipefail

# Packaging script for Jetson tarball (aarch64). Generates a tar.gz with binaries + systemd unit.
# Usage:
#   packaging/jetson/pack.sh
#   ARCH=aarch64 PROFILE=release packaging/jetson/pack.sh

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ARCH="${ARCH:-$(uname -m)}"
PROFILE="${PROFILE:-release}"
FEATURES="${FEATURES:-tui daemon cli}"
PROFILE_FLAG="--profile ${PROFILE}"

VERSION="${VERSION:-$(grep '^version' "$ROOT/Cargo.toml" | head -n1 | cut -d '\"' -f2)}"
STAGE="$ROOT/target/package/jetsonscope-${VERSION}-${ARCH}"
TARBALL="$ROOT/target/jetsonscope-${VERSION}-${ARCH}.tar.gz"

echo "📦 Packaging JetsonScope version ${VERSION} for arch ${ARCH} (profile: ${PROFILE})"
rm -rf "$STAGE"
mkdir -p "$STAGE/usr/local/bin" "$STAGE/etc/systemd/system"

echo "🔨 Building binaries..."
cargo build ${PROFILE_FLAG} --features "${FEATURES}" --bin jscope --bin jscoped --bin jscopectl

echo "📥 Staging files..."
cp "$ROOT/target/${PROFILE}/jscope" "$STAGE/usr/local/bin/jscope"
cp "$ROOT/target/${PROFILE}/jscoped" "$STAGE/usr/local/bin/jscoped"
cp "$ROOT/target/${PROFILE}/jscopectl" "$STAGE/usr/local/bin/jscopectl"
install -m 0644 "$ROOT/install/jscoped.service" "$STAGE/etc/systemd/system/jscoped.service"

cat > "$STAGE/README-package.md" <<'EOF'
JetsonScope - Jetson Package
============================

Contents:
- /usr/local/bin/jscope (TUI)
- /usr/local/bin/jscoped (daemon)
- /usr/local/bin/jscopectl (CLI)
- /etc/systemd/system/jscoped.service

Install:
1) As root (or sudo):
   tar -C / -xzf jetsonscope-<version>-<arch>.tar.gz
2) Systemd:
   sudo systemctl daemon-reload
   sudo systemctl enable jscoped
   sudo systemctl start jscoped
3) Launch the TUI:
   jscope

Notes:
- Change JETSONSCOPE_SOCKET_PATH if you need a different socket.
- If you use auth, export JETSONSCOPE_AUTH_TOKEN in the service.
EOF

echo "🗜️  Creating tarball..."
tar -C "$STAGE" -czf "$TARBALL" .

echo "✅ Done: $TARBALL"
