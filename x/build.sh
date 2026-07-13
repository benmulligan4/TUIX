#!/bin/bash
# Build locally cloned dashboards and applications (source "local").
# Looks for Cargo.toml in downloads/dashboards/{name} and downloads/applications/{name}.
# Runs cargo build --release for each.
# Logs actions to tuix.log.

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

DASHBOARDS_JSON="$PROJECT_ROOT/config/dashboards.json"
APPS_JSON="$PROJECT_ROOT/config/apps.json"
DASHBOARDS_DIR="$PROJECT_ROOT/downloads/dashboards"
APPS_DIR="$PROJECT_ROOT/downloads/applications"
LOG_FILE="$PROJECT_ROOT/tuix.log"

log_entry() {
    local level="$1"
    local message="$2"
    local timestamp
    timestamp=$(date "+%Y-%m-%d %H:%M:%S")
    echo "[$timestamp] [$level] $message" >> "$LOG_FILE"
}

log_entry "SCRIPT" "build.sh started"

build_from_json() {
    local json_file="$1"
    local downloads_dir="$2"
    local category="$3"

    if [ ! -f "$json_file" ]; then
        echo "[$category] Config not found: $json_file — skipping."
        log_entry "ERROR" "build.sh: Config not found: $json_file"
        return
    fi

    # Extract entries that have source "local"
    local entries
    entries=$(python3 -c "
import json, sys
with open('$json_file') as f:
    data = json.load(f)
for name, meta in data.items():
    source = meta.get('source', '')
    if source == 'local':
        print(name)
" 2>/dev/null || true)

    if [ -z "$entries" ]; then
        echo "[$category] No local entries to build."
        log_entry "SCRIPT" "build.sh: No local entries for $category"
        return
    fi

    while IFS= read -r name; do
        local target="$downloads_dir/$name"
        local cargo_toml="$target/Cargo.toml"

        if [ ! -d "$target" ]; then
            echo "[$category] $name not cloned yet — run clone.sh first. Skipping."
            log_entry "ERROR" "build.sh: $name not found at $target, skipping"
            continue
        fi

        if [ ! -f "$cargo_toml" ]; then
            echo "[$category] $name has no Cargo.toml — skipping."
            log_entry "ERROR" "build.sh: No Cargo.toml found for $name at $target"
            continue
        fi

        echo "[$category] Building $name ..."
        if cargo build --release --manifest-path "$cargo_toml"; then
            echo "[$category] $name built successfully."
            log_entry "SCRIPT" "build.sh: Built $name (release) at $target"
        else
            echo "[$category] ERROR: Failed to build $name"
            log_entry "ERROR" "build.sh: Failed to build $name at $target"
        fi
    done <<< "$entries"
}

echo "=== TUIX Build Script (Local) ==="
echo ""

build_from_json "$DASHBOARDS_JSON" "$DASHBOARDS_DIR" "Dashboards"
echo ""
build_from_json "$APPS_JSON" "$APPS_DIR" "Applications"

echo ""
echo "=== Done ==="
log_entry "SCRIPT" "build.sh completed"
