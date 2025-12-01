#!/usr/bin/env bash
set -euo pipefail

# Helper to list or set controls via jetsonscopectl.
# Usage:
#   ./scripts/jetsonscope-control.sh list
#   ./scripts/jetsonscope-control.sh set fan 70
#   ./scripts/jetsonscope-control.sh set nvpmodel MAXN
#   ./scripts/jetsonscope-control.sh set jetson_clocks on

ctl_bin="${JETSONSCOPECTL_BIN:-jscopectl}"
action="${1:-list}"

case "$action" in
    list)
        exec "$ctl_bin" list
        ;;
    set)
        ctrl="${2:-}"
        val="${3:-}"
        if [ -z "$ctrl" ] || [ -z "$val" ]; then
            echo "Usage: $0 set <control> <value>"
            exit 1
        fi
        exec "$ctl_bin" set "$ctrl" "$val"
        ;;
    *)
        echo "Usage: $0 [list | set <control> <value>]"
        exit 1
        ;;
esac
