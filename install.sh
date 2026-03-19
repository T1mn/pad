#!/bin/bash
# Universal installer for tmux-code-kanban
# Supports: uv, pip, and fallback to source

set -e

REPO="T1mn/tmux_kanban"
PACKAGE="tmux-code-kanban"
RUST_PACKAGE="tmux-kanban-core"

echo "=============================================="
echo "Tmux Code Kanban Installer"
echo "=============================================="

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Detect OS
OS="unknown"
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="linux"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    OS="macos"
fi

# Check Python version
check_python() {
    if command -v python3 &> /dev/null; then
        PYTHON_VERSION=$(python3 --version 2>&1 | awk '{print $2}')
        PYTHON_MAJOR=$(echo $PYTHON_VERSION | cut -d. -f1)
        PYTHON_MINOR=$(echo $PYTHON_VERSION | cut -d. -f2)
        
        if [ "$PYTHON_MAJOR" -ge 3 ] && [ "$PYTHON_MINOR" -ge 9 ]; then
            echo -e "${GREEN}✓ Python $PYTHON_VERSION found${NC}"
            return 0
        else
            echo -e "${RED}✗ Python 3.9+ required, found $PYTHON_VERSION${NC}"
            return 1
        fi
    else
        echo -e "${RED}✗ Python 3 not found${NC}"
        return 1
    fi
}

# Check Rust (optional, for performance)
check_rust() {
    if command -v rustc &> /dev/null; then
        RUST_VERSION=$(rustc --version 2>&1 | awk '{print $2}')
        echo -e "${GREEN}✓ Rust $RUST_VERSION found${NC}"
        return 0
    else
        echo -e "${YELLOW}! Rust not found (optional, for better performance)${NC}"
        return 1
    fi
}

# Install via uv
install_uv() {
    echo -e "${GREEN}Installing via uv...${NC}"
    if command -v uv &> /dev/null; then
        uv tool install $PACKAGE
        if check_rust; then
            echo -e "${GREEN}Installing Rust core for better performance...${NC}"
            pip install $RUST_PACKAGE
        fi
        return 0
    fi
    return 1
}

# Install via pip
install_pip() {
    echo -e "${GREEN}Installing via pip...${NC}"
    
    # Detect China network (simple check)
    CHINA_MIRROR=""
    if curl -s --connect-timeout 3 https://pypi.tuna.tsinghua.edu.cn/simple > /dev/null 2>&1; then
        echo -e "${YELLOW}China network detected, using Tsinghua mirror${NC}"
        CHINA_MIRROR="-i https://pypi.tuna.tsinghua.edu.cn/simple"
    fi
    
    pip3 install $PACKAGE $CHINA_MIRROR
    
    if check_rust; then
        echo -e "${GREEN}Installing Rust core for better performance...${NC}"
        pip3 install $RUST_PACKAGE $CHINA_MIRROR
    fi
}

# Install from source
install_source() {
    echo -e "${YELLOW}Installing from source...${NC}"
    
    TEMP_DIR=$(mktemp -d)
    cd $TEMP_DIR
    
    git clone --depth 1 https://github.com/$REPO.git
    cd tmux_kanban
    
    pip3 install -e .
    
    if check_rust; then
        echo -e "${GREEN}Building Rust core...${NC}"
        cd rust
        pip3 install maturin
        maturin build --release
        pip3 install target/wheels/*.whl
    fi
    
    cd /
    rm -rf $TEMP_DIR
}

# Main installation flow
main() {
    echo ""
    
    # Check Python
    if ! check_python; then
        echo -e "${RED}Please install Python 3.9+ first${NC}"
        echo "  Ubuntu/Debian: sudo apt install python3 python3-pip"
        echo "  macOS: brew install python3"
        exit 1
    fi
    
    # Try installation methods in order
    if command -v uv &> /dev/null; then
        echo -e "${GREEN}Detected uv, using uv tool install${NC}"
        install_uv
    elif command -v pip3 &> /dev/null; then
        echo -e "${GREEN}Detected pip3${NC}"
        install_pip
    else
        echo -e "${YELLOW}No package manager found, installing from source${NC}"
        install_source
    fi
    
    # Verify installation
    echo ""
    echo "Verifying installation..."
    if command -v pad &> /dev/null; then
        echo -e "${GREEN}✓ Installation successful!${NC}"
        echo ""
        echo "Usage:"
        echo "  pad            # Launch interactive TUI"
        echo "  pad tk         # Launch TUI (alias)"
        echo "  pad --help     # Show help"
        echo ""
        echo "Quick start:"
        echo "  1. Run: pad"
        echo "  2. Use ↑/↓ or j/k to navigate"
        echo "  3. Press Enter to open a panel"
        echo "  4. Press q to quit"
    else
        echo -e "${RED}✗ Installation may have failed${NC}"
        echo "Please check the error messages above"
        exit 1
    fi
}

# Check for help flag
if [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    echo "Tmux Code Kanban Installer"
    echo ""
    echo "This script will:"
    echo "  1. Check Python 3.9+ is installed"
    echo "  2. Detect the best package manager (uv > pip)"
    echo "  3. Install tmux-code-kanban"
    echo "  4. Optionally install Rust core for better performance"
    echo ""
    echo "Options:"
    echo "  --help, -h     Show this help"
    echo ""
    echo "Manual installation:"
    echo "  pip install tmux-code-kanban"
    exit 0
fi

main
