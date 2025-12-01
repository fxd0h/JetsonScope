#!/usr/bin/env bash
set -euo pipefail

# Simple service helper for JetsonScope daemon.
# Usage: ./scripts/jetsonscope-service.sh [start|stop|restart|status|foreground]

cmd="${1:-status}"
daemon_bin="${JETSONSCOPE_DAEMON_BIN:-jscoped}"
socket="${JETSONSCOPE_SOCKET_PATH:-${TEGRA_SOCKET_PATH:-/tmp/jetsonscope.sock}}"
service_name="jscoped"

have_systemctl() {
    command -v systemctl >/dev/null 2>&1
}

case "$cmd" in
    start)
        if have_systemctl; then
            sudo systemctl start "$service_name"
        else
            echo "Starting $daemon_bin (no systemd detected)..."
            sudo "$daemon_bin" &
        fi
        ;;
    stop)
        if have_systemctl; then
            sudo systemctl stop "$service_name"
        else
            pkill -f "$daemon_bin" || true
        fi
        ;;
    restart)
        if have_systemctl; then
            sudo systemctl restart "$service_name"
        else
            pkill -f "$daemon_bin" || true
            sudo "$daemon_bin" &
        fi
        ;;
    status)
        echo "Socket: $socket"
        if have_systemctl; then
            sudo systemctl status "$service_name" --no-pager || true
        else
            pgrep -fl "$daemon_bin" || echo "$daemon_bin not running"
        fi
        ;;
    foreground)
        echo "Starting $daemon_bin in foreground (Ctrl+C to stop)..."
        exec sudo "$daemon_bin"
        ;;
    *)
        echo "Usage: $0 [start|stop|restart|status|foreground]"
        exit 1
        ;;
esac
