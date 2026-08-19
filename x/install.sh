#!/bin/bash
# Install third-party dashboards and applications with source "global" via cargo install.
# Reads config/dashboards.json and config/apps.json for entries with source "global".
# Logs actions to tuix.log.

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

DASHBOARDS_JSON="$PROJECT_ROOT/config/dashboards.json"
APPS_JSON="$PROJECT_ROOT/config/apps.json"
LOG_FILE="$PROJECT_ROOT/tuix.log"

log_entry() {
    local level="$1"
    local message="$2"
    local timestamp
    timestamp=$(date "+%Y-%m-%d %H:%M:%S")
    echo "[$timestamp] [$level] $message" >> "$LOG_FILE"
}

log_entry "SCRIPT" "install.sh started"

install_from_json() {
    local json_file="$1"
    local category="$2"

    if [ ! -f "$json_file" ]; then
        echo "[$category] Config not found: $json_file — skipping."
        log_entry "ERROR" "install.sh: Config not found: $json_file"
        return
    fi

    # Extract entries that have source "global" and a cmd field
    local entries
    entries=$(python3 -c "
import json, sys
with open('$json_file') as f:
    data = json.load(f)
for name, meta in data.items():
    source = meta.get('source', '')
    cmd = meta.get('cmd', [])
    if source == 'global' and cmd:
        print(f'{name}|{cmd[0]}')
" 2>/dev/null || true)

    if [ -z "$entries" ]; then
        echo "[$category] No global third-party entries found."
        log_entry "SCRIPT" "install.sh: No global entries for $category"
        return
    fi

    while IFS='|' read -r name crate_name; do
        echo "[$category] Installing $name ($crate_name) via cargo install ..."
        if cargo install "$crate_name" 2>/dev/null; then
            echo "[$category] $name installed successfully."
            log_entry "SCRIPT" "install.sh: Installed $name ($crate_name) globally via cargo install"
        else
            echo "[$category] ERROR: Failed to install $name ($crate_name)"
            log_entry "ERROR" "install.sh: Failed to cargo install $crate_name"
        fi
    done <<< "$entries"
}

echo "=== TUIX Install Script (Global) ==="
echo ""

install_from_json "$DASHBOARDS_JSON" "Dashboards"
echo ""
install_from_json "$APPS_JSON" "Applications"

echo ""
echo "=== Done ==="
log_entry "SCRIPT" "install.sh completed"
