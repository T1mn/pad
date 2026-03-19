#!/bin/bash
# Publish script for tmux-ai-kanban

set -e

echo "=============================================="
echo "Publishing tmux-ai-kanban"
echo "=============================================="

# Clean previous builds
rm -rf dist/ build/

# Build
echo "Building package..."
python3 -m build

# Check
echo "Checking package..."
python3 -m twine check dist/*

# Upload to PyPI
echo "Uploading to PyPI..."
python3 -m twine upload dist/*

echo ""
echo "=============================================="
echo "Published successfully!"
echo "=============================================="
echo ""
echo "Install with:"
echo "  pip install tmux-ai-kanban"
echo ""
echo "Or use China mirror:"
echo "  pip install tmux-ai-kanban -i https://pypi.tuna.tsinghua.edu.cn/simple"
