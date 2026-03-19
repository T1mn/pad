#!/bin/bash
# Build script for Rust core module

set -e

echo "=============================================="
echo "Building tmux-kanban-core (Rust)"
echo "=============================================="

cd rust

# Check if maturin is installed
if ! command -v maturin &> /dev/null; then
    echo "Installing maturin..."
    pip install maturin
fi

# Build the wheel
echo "Building wheel..."
maturin build --release

echo ""
echo "=============================================="
echo "Build complete!"
echo "=============================================="
echo ""
echo "To install the Rust extension:"
echo "  pip install rust/target/wheels/*.whl"
echo ""
echo "Or install in development mode:"
echo "  cd rust && maturin develop"
