#!/bin/bash
# Installer for pad - Tmux Agent Panel Manager
set -e

echo "=============================================="
echo "  pad - Tmux Agent Panel Manager"
echo "=============================================="

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

check_rust() {
    if command -v cargo &> /dev/null; then
        RUST_VERSION=$(rustc --version 2>&1 | awk '{print $2}')
        echo -e "${GREEN}✓ Rust $RUST_VERSION found${NC}"
        return 0
    else
        echo -e "${RED}✗ Rust/Cargo not found${NC}"
        echo "  Install: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        return 1
    fi
}

install_from_source() {
    echo ""
    echo "Building from source..."
    cd rust-tui
    cargo build --release

    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
    cp target/release/pad "$INSTALL_DIR/pad"

    echo -e "${GREEN}✓ Installed to $INSTALL_DIR/pad${NC}"

    # Check if in PATH
    if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
        echo -e "${YELLOW}! Add $INSTALL_DIR to your PATH:${NC}"
        echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    fi
}

main() {
    echo ""

    if ! check_rust; then
        exit 1
    fi

    install_from_source

    echo ""
    if command -v pad &> /dev/null; then
        echo -e "${GREEN}✓ Installation successful!${NC}"
        echo ""
        echo "Usage:"
        echo "  pad            Launch interactive TUI"
        echo "  pad --help     Show help"
        echo ""
        echo "Quick start:"
        echo "  1. Start an AI agent in tmux (claude, codex, kimi-cli)"
        echo "  2. Run: pad"
        echo "  3. Use j/k to navigate, Enter to attach"
        echo "  4. F12 or Ctrl+Q to detach back to pad"
        echo "  5. Press q to quit"
    else
        echo -e "${GREEN}Build successful! Binary at rust-tui/target/release/pad${NC}"
    fi
}

if [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    echo "pad Installer"
    echo ""
    echo "Requirements: Rust toolchain (cargo)"
    echo ""
    echo "This script will:"
    echo "  1. Build pad from source with cargo"
    echo "  2. Install to ~/.local/bin/pad"
    echo ""
    echo "Manual build:"
    echo "  cd rust-tui && cargo build --release"
    exit 0
fi

main
