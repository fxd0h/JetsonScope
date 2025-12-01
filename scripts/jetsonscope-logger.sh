#!/usr/bin/env bash
set -euo pipefail

# Periodically pull stats and print JSON/CBOR to stdout (similar to jtop_logger).
# Usage: ./scripts/jetsonscope-logger.sh [interval_ms] [json|cbor]

interval_ms="${1:-1000}"
proto="${2:-json}"
ctl_bin="${JETSONSCOPECTL_BIN:-jscopectl}"
socket="${JETSONSCOPE_SOCKET_PATH:-${TEGRA_SOCKET_PATH:-/tmp/jetsonscope.sock}}"

if [ "$proto" = "cbor" ]; then
    export JETSONSCOPE_PROTO=cbor
fi

echo "Logging stats every ${interval_ms}ms from $socket (proto=$proto)..."
while true; do
    if ! out="$("$ctl_bin" stats 2>/dev/null)"; then
        echo "Error: could not read stats (socket?)" >&2
    else
        echo "$out"
    fi
    sleep "$(awk "BEGIN {print $interval_ms/1000}")"
done
