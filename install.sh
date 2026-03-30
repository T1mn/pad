#!/bin/bash
# Installer for PAD (Panel for Agent Development)
# Supports: pre-built binaries (fast) or building from source (fallback)
set -e

REPO="T1mn/pad"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

get_arch() {
    local arch=$(uname -m)
    case "$arch" in
        x86_64|amd64)  echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *)       echo "unsupported" ;;
    esac
}

get_os() {
    local os=$(uname -s | tr '[:upper:]' '[:lower:]')
    case "$os" in
        linux)   echo "linux" ;;
        darwin)  echo "macos" ;;
        *)       echo "unsupported" ;;
    esac
}

check_tmux() {
    command -v tmux >/dev/null 2>&1
}

check_rust() {
    if command -v cargo &> /dev/null; then
        RUST_VERSION=$(rustc --version 2>&1 | awk '{print $2}')
        echo -e "${GREEN}✓ Rust $RUST_VERSION found${NC}"
        return 0
    else
        return 1
    fi
}

install_from_binary() {
    local version="${VERSION:-latest}"
    local os=$(get_os)
    local arch=$(get_arch)
    
    if [ "$arch" = "unsupported" ] || [ "$os" = "unsupported" ]; then
        return 1
    fi
    
    # macOS universal binary
    if [ "$os" = "macos" ]; then
        arch="universal"
    fi
    
    echo -e "${BLUE}Trying to download pre-built binary...${NC}"
    echo "  Platform: ${os}/${arch}"
    
    if [ "$version" = "latest" ]; then
        version=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
        if [ -z "$version" ]; then
            echo -e "${YELLOW}  Could not fetch latest version${NC}"
            return 1
        fi
    fi
    
    local filename="pad-${version}-${os}-${arch}.tar.gz"
    local url="https://github.com/$REPO/releases/download/${version}/${filename}"
    
    echo "  Version:  ${version}"
    echo "  URL:      ${url}"
    
    local tmp_dir=$(mktemp -d)
    trap "rm -rf $tmp_dir" EXIT
    
    if ! curl -fsSL "$url" -o "$tmp_dir/pad.tar.gz" 2>/dev/null; then
        echo -e "${YELLOW}  Download failed (binary may not exist for this platform)${NC}"
        return 1
    fi
    
    echo -e "${BLUE}Extracting...${NC}"
    tar -xzf "$tmp_dir/pad.tar.gz" -C "$tmp_dir"
    
    mkdir -p "$INSTALL_DIR"
    mv "$tmp_dir/pad" "$INSTALL_DIR/pad"
    chmod +x "$INSTALL_DIR/pad"
    
    echo -e "${GREEN}✓ Installed to $INSTALL_DIR/pad${NC}"
    return 0
}

install_from_source() {
    echo ""
    echo -e "${BLUE}Building from source...${NC}"
    
    if ! check_rust; then
        echo -e "${RED}✗ Rust/Cargo not found${NC}"
        echo ""
        echo "Install Rust:"
        echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo ""
        echo "Or download pre-built binary from:"
        echo "  https://github.com/$REPO/releases"
        return 1
    fi
    
    cd rust-tui
    cargo build --profile dist
    
    mkdir -p "$INSTALL_DIR"
    cp target/dist/pad "$INSTALL_DIR/pad"
    
    echo -e "${GREEN}✓ Installed to $INSTALL_DIR/pad${NC}"
}

main() {
    echo "=============================================="
    echo "  PAD - Panel for Agent Development"
    echo "=============================================="
    echo ""
    
    # 尝试二进制安装（快速）
    if install_from_binary; then
        : # success
    else
        # 回退到源码安装
        echo ""
        echo -e "${YELLOW}Falling back to building from source...${NC}"
        install_from_source
    fi
    
    # 检查 PATH
    if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
        echo ""
        echo -e "${YELLOW}! Add to your shell config:${NC}"
        echo "   export PATH=\"$INSTALL_DIR:\$PATH\""
    fi
    
    echo ""
    if command -v pad &> /dev/null || [ -x "$INSTALL_DIR/pad" ]; then
        echo -e "${GREEN}✓ Installation successful!${NC}"
        echo ""
        echo "Usage:"
        echo "  pad            Launch interactive TUI"
        echo "  pad --help     Show help"
        echo ""
        if ! check_tmux; then
            echo -e "${YELLOW}! tmux is not installed${NC}"
            echo "  pad requires tmux at runtime."
            echo "  On WSL2, install tmux inside WSL before running pad."
            echo ""
        fi
        echo "Quick start:"
        echo "  1. Start an AI agent in tmux (claude, codex, kimi-cli)"
        echo "  2. Run: pad"
        echo "  3. Use j/k to navigate, Enter to attach"
        echo "  4. F12 or Ctrl+Q to detach back to pad"
        echo "  5. Press q to quit"
    else
        echo -e "${RED}Installation may have failed${NC}"
    fi
}

if [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    echo "PAD Installer"
    echo ""
    echo "This script will:"
    echo "  1. Try to download pre-built binary (fast, ~1.5MB)"
    echo "  2. Fall back to building from source if needed"
    echo ""
    echo "Environment variables:"
    echo "  INSTALL_DIR    Installation directory (default: ~/.local/bin)"
    echo "  VERSION        Specific version to install (default: latest)"
    echo ""
    echo "Runtime requirement:"
    echo "  tmux must be installed in the same environment as pad."
    echo "  On WSL2, install and run both tmux and pad inside WSL."
    echo ""
    echo "Manual install:"
    echo "  Build from source: cd rust-tui && cargo build --profile dist"
    echo "  Pre-built binaries: https://github.com/$REPO/releases"
    exit 0
fi

main
