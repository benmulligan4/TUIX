#!/bin/bash
# Clone third-party dashboards and applications with source "local".
# Clones to downloads/dashboards/{name} and downloads/applications/{name}.
# Excludes .git and .github directories from cloned repos.
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

log_entry "SCRIPT" "clone.sh started"

# Ensure download directories exist
mkdir -p "$DASHBOARDS_DIR"
mkdir -p "$APPS_DIR"

clone_from_json() {
    local json_file="$1"
    local install_dir="$2"
    local category="$3"

    if [ ! -f "$json_file" ]; then
        echo "[$category] Config not found: $json_file — skipping."
        log_entry "ERROR" "clone.sh: Config not found: $json_file"
        return
    fi

    # Extract entries that have source "local" and a "repository" field
    local entries
    entries=$(python3 -c "
import json, sys
with open('$json_file') as f:
    data = json.load(f)
for name, meta in data.items():
    source = meta.get('source', '')
    repo = meta.get('repository', '')
    if source == 'local' and repo:
        print(f'{name}|{repo}')
" 2>/dev/null || true)

    if [ -z "$entries" ]; then
        echo "[$category] No local third-party entries found."
        log_entry "SCRIPT" "clone.sh: No local entries for $category"
        return
    fi

    while IFS='|' read -r name repo; do
        local target="$install_dir/$name"
        if [ -d "$target" ]; then
            echo "[$category] $name already cloned — skipping."
            log_entry "SCRIPT" "clone.sh: $name already exists, skipped"
        else
            echo "[$category] Cloning $name from $repo ..."
            if git clone "$repo" "$target" 2>/dev/null; then
                # Remove .git and .github directories
                rm -rf "$target/.git"
                rm -rf "$target/.github"
                echo "[$category] $name cloned successfully."
                log_entry "SCRIPT" "clone.sh: Cloned $name from $repo to $target"
            else
                echo "[$category] ERROR: Failed to clone $name from $repo"
                log_entry "ERROR" "clone.sh: Failed to clone $name from $repo"
            fi
        fi
    done <<< "$entries"
}

echo "=== TUIX Clone Script ==="
echo ""

clone_from_json "$DASHBOARDS_JSON" "$DASHBOARDS_DIR" "Dashboards"
echo ""
clone_from_json "$APPS_JSON" "$APPS_DIR" "Applications"

echo ""
echo "=== Done ==="
log_entry "SCRIPT" "clone.sh completed"
