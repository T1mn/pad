# Tmux AI Kanban

Manage and monitor AI coding assistants (Claude, Codex, Kimi) running in tmux panes.

## Features

- 🔍 **Auto-detect** AI panels in tmux sessions
- 📊 **TUI interface** with keyboard navigation
- 🌿 **Git integration** - show branch, commit, status
- ⚡ **Activity indicators** - spinner for active panels
- 🎯 **Filter & Search** by AI type, session, directory
- 🚀 **High Performance** - Rust core for 2-3x speedup

## Installation

### From PyPI (Recommended)

```bash
pip install tmux-ai-kanban
```

### China Mirror (国内镜像)

```bash
# 清华镜像
pip install tmux-ai-kanban -i https://pypi.tuna.tsinghua.edu.cn/simple

# 阿里云镜像
pip install tmux-ai-kanban -i https://mirrors.aliyun.com/pypi/simple/

# 豆瓣镜像
pip install tmux-ai-kanban -i https://pypi.doubanio.com/simple/
```

### From GitHub

```bash
pip install git+https://github.com/T1mn/tmux_kanban.git
```

### From Gitee (国内)

```bash
pip install git+https://gitee.com/yourname/tmux_kanban.git
```

## Usage

### Interactive TUI Mode

```bash
# Launch TUI
tak tui

# or
ai-kanban tui

# Filter by AI type
tak tui -f claude
```

**Keyboard Shortcuts**:
- `↑/↓` or `j/k` - Navigate
- `1-9` - Jump to panel
- `Enter` - Open panel in popup (zoomed)
- `/` - Search
- `r` - Refresh
- `q` - Quit

### CLI Mode

```bash
# List all AI panels
ai-kanban list

# Watch mode (auto-refresh)
ai-kanban list --watch

# Filter by AI type
ai-kanban list --filter claude

# Show summary
ai-kanban summary

# Show pane content
ai-kanban show <pane_id>

# Jump to a pane
ai-kanban jump <session> <window> <pane>
```

## Rust Core (高性能模式)

Install Rust core for 2-3x performance improvement:

```bash
pip install tmux-kanban-core
```

Verify:
```python
from tmux_ai_kanban.detector_rust import is_rust_available
print(is_rust_available())  # True
```

## Supported AI Tools

- 🟣 Claude (claude)
- 🔵 OpenAI Codex (codex)
- 🟢 Kimi (kimi)

## Requirements

- Python 3.9+
- tmux 3.0+
- Linux/macOS

## License

MIT
