#!/bin/bash
set -e

# Synthread GUI — build & run script

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
GUI_DIR="$SCRIPT_DIR/frontends/gui"
BUILD_DIR="$GUI_DIR/build"

cd "$SCRIPT_DIR"

# Pull latest changes
echo "=== Pulling latest ==="
git pull --ff-only 2>/dev/null || true

# Build
echo ""
echo "=== Building Qt6 GUI ==="
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"
cd "$BUILD_DIR"
cmake .. -DCMAKE_BUILD_TYPE=Release
make -j$(nproc)
cd "$SCRIPT_DIR"

echo ""
echo "=== Starting Synthread GUI ==="
exec "$BUILD_DIR/synthread-gui"
