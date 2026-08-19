#!/bin/bash
# TUIX Setup Script — runs install, clone, and build in order.
# 1. install.sh — cargo install for global entries
# 2. clone.sh — clone local entries to downloads/
# 3. build.sh — cargo build --release for local entries
# Logs actions to tuix.log.

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_FILE="$PROJECT_ROOT/tuix.log"

log_entry() {
    local level="$1"
    local message="$2"
    local timestamp
    timestamp=$(date "+%Y-%m-%d %H:%M:%S")
    echo "[$timestamp] [$level] $message" >> "$LOG_FILE"
}

log_entry "SCRIPT" "setup.sh started"

echo "========================================"
echo "  TUIX Setup"
echo "========================================"
echo ""

# Step 1: Install global entries
echo "--- Step 1/3: Installing global entries ---"
echo ""
bash "$SCRIPT_DIR/install.sh"
echo ""

# Step 2: Clone local entries
echo "--- Step 2/3: Cloning local entries ---"
echo ""
bash "$SCRIPT_DIR/clone.sh"
echo ""

# Step 3: Build local entries
echo "--- Step 3/3: Building local entries ---"
echo ""
bash "$SCRIPT_DIR/build.sh"
echo ""

echo "========================================"
echo "  TUIX Setup Complete"
echo "========================================"
log_entry "SCRIPT" "setup.sh completed"
