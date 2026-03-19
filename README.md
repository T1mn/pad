# Tmux AI Kanban

Manage and monitor AI coding assistants (Claude, Codex, Kimi) running in tmux panes.

## Features

- 🔍 **Auto-detect** AI panels in tmux sessions
- 📊 **TUI interface** with keyboard navigation
- 🌿 **Git integration** - show branch, commit, status
- ⚡ **Activity indicators** - spinner for active panels
- 🎯 **Filter & Search** by AI type, session, directory
- 🚀 **High Performance** - Rust core for 2-3x speedup

## Quick Install

### One-liner (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/T1mn/tmux_kanban/main/install.sh | bash
```

### UV (Fastest)

```bash
uv tool install tmux-ai-kanban
```

### Pip

```bash
# Global
pip install tmux-ai-kanban

# China mirror
pip install tmux-ai-kanban -i https://pypi.tuna.tsinghua.edu.cn/simple
```

### With Rust Core (Best Performance)

```bash
pip install tmux-ai-kanban tmux-kanban-core
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

## Performance

| Mode | 10 panes | 50 panes |
|------|----------|----------|
| Python | ~800ms | ~4s |
| **Rust** | ~300ms | ~1.2s |

Enable Rust core for 2-3x speedup:
```bash
pip install tmux-kanban-core
```

## Supported AI Tools

- 🟣 Claude (claude)
- 🔵 OpenAI Codex (codex)
- 🟢 Kimi (kimi)

## Requirements

- Python 3.9+
- tmux 3.0+
- Linux/macOS

## Development

```bash
git clone https://github.com/T1mn/tmux_kanban.git
cd tmux_kanban
pip install -e .

# Build Rust core (optional)
cd rust
maturin build --release
pip install target/wheels/*.whl
```

## License

MIT
