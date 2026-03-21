# pad

Tmux Agent Panel Manager — monitor and manage AI coding assistants (Claude, Codex, Kimi, Gemini, OpenCode, Aider, Cursor) running in tmux.

## Features

- 🔍 Auto-detect AI agent panels across all tmux sessions
- 📊 TUI with keyboard navigation and live preview
- 🌿 Git integration — branch, commit, changed files
- ⚡ Activity detection — spinners, "thinking" markers
- 🔎 Search and filter panels
- 🌲 File tree explorer with syntax-highlighted preview
- 🚀 PTY attach — jump into any panel with F12/Ctrl+Q to return
- 🎨 Theme selector (Dracula, Nord, Catppuccin, etc.)
- 🤖 Agent launcher — start new AI agents from the tree view

## Install

Requires: Rust toolchain, tmux, Linux/macOS.

```bash
# From source
git clone https://github.com/T1mn/tmux_kanban.git
cd tmux_kanban/rust-tui
cargo build --release

# Install to ~/.local/bin
cp target/release/pad ~/.local/bin/

# Or use the install script
./install.sh
```

## Usage

```bash
pad              # Launch TUI
pad --help       # Show help
pad --version    # Show version
```

## Key Bindings

| Key | Action |
|-----|--------|
| `j/k` or `↑/↓` | Navigate panels |
| `1-9` | Jump to panel |
| `Enter` | Attach to panel |
| `F12` / `Ctrl+Q` | Detach back to pad |
| `/` | Search panels |
| `?` | Help |
| `t` | Toggle file tree |
| `T` | Open tree at ~/ |
| `Space` | Expand/collapse directory |
| `c` | Create new session |
| `d` | Delete panel |
| `r` | Refresh |
| `F1` | Settings |
| `q` | Quit |

## Supported Agents

- 🟣 Claude (`claude`)
- 🔵 Codex (`codex`)
- 🟢 Kimi (`kimi-cli`)
- 🔷 Gemini (`gemini-cli`)
- 🟠 OpenCode (`opencode`)
- 🟡 Aider (`aider`)
- 🟤 Cursor (`cursor`)

## License

MIT
