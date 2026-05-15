#!/bin/bash
set -e

# Synthread build script
# Builds Rust backend + Qt6 GUI frontend

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Synthread Build ==="
echo ""

# ── Rust backend ──
echo "[1/2] Building Rust backend..."
source "$HOME/.cargo/env" 2>/dev/null || true
cargo build --release
echo "  → target/release/synthread"
echo ""

# ── Qt6 GUI ──
echo "[2/2] Building Qt6 GUI..."
GUI_DIR="$SCRIPT_DIR/frontends/gui"
BUILD_DIR="$GUI_DIR/build"

if [ -f "$BUILD_DIR/synthread-gui" ]; then
    echo "  GUI already built, skipping..."
else
    mkdir -p "$BUILD_DIR"
    cd "$BUILD_DIR"
    cmake .. -DCMAKE_BUILD_TYPE=Release && make -j$(nproc)
    cd "$SCRIPT_DIR"
fi
echo "  → $BUILD_DIR/synthread-gui"
echo ""

echo "=== Build complete ==="
echo ""
echo "Run backend:  cargo run --release -- --mode headless"
echo "Run GUI:      $BUILD_DIR/synthread-gui"
echo "Run TUI:      cargo run --release -- --mode tui"
