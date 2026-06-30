#!/bin/bash
# Clone third-party dashboards and applications that aren't already installed.
# Reads config/dashboards.json and config/apps.json for entries with "repository" field.

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

DASHBOARDS_JSON="$PROJECT_ROOT/config/dashboards.json"
APPS_JSON="$PROJECT_ROOT/config/apps.json"
DASHBOARDS_INSTALLED="$PROJECT_ROOT/src/dashboards/installed"
APPS_INSTALLED="$PROJECT_ROOT/src/applications/installed"

# Ensure installed directories exist
mkdir -p "$DASHBOARDS_INSTALLED"
mkdir -p "$APPS_INSTALLED"

clone_from_json() {
    local json_file="$1"
    local install_dir="$2"
    local category="$3"

    if [ ! -f "$json_file" ]; then
        echo "[$category] Config not found: $json_file — skipping."
        return
    fi

    # Extract entries that have a "repository" field
    local entries
    entries=$(python3 -c "
import json, sys
with open('$json_file') as f:
    data = json.load(f)
for name, meta in data.items():
    repo = meta.get('repository', '')
    if repo:
        print(f'{name}|{repo}')
" 2>/dev/null || true)

    if [ -z "$entries" ]; then
        echo "[$category] No third-party entries found."
        return
    fi

    while IFS='|' read -r name repo; do
        local target="$install_dir/$name"
        if [ -d "$target" ]; then
            echo "[$category] $name already installed — skipping."
        else
            echo "[$category] Cloning $name from $repo ..."
            git clone "$repo" "$target"
            # Remove .github directory from cloned repo
            rm -rf "$target/.github"
            echo "[$category] $name installed successfully."
        fi
    done <<< "$entries"
}

echo "=== TUIX Clone Script ==="
echo ""

clone_from_json "$DASHBOARDS_JSON" "$DASHBOARDS_INSTALLED" "Dashboards"
echo ""
clone_from_json "$APPS_JSON" "$APPS_INSTALLED" "Applications"

echo ""
echo "=== Done ==="
