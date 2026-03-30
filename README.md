# PAD

`pad` is the CLI for PAD: Panel for Agent Development.

PAD is a tmux-first workspace for monitoring, previewing, and jumping between AI coding agents such as Claude, Codex, Kimi, Gemini, OpenCode, Aider, and Cursor.

Control surface for AI coding agents in tmux.

PAD is built for the moment when you have more than one agent, more than one session, and you need a fast way to see what is running, what just moved, and where to jump next.

With PAD you can:

- See live and historical agent sessions in one place
- Preview recent turns before attaching
- Jump into any pane and return safely
- Keep session metadata, archive state, and relay settings together
- Work across Codex, Claude, Gemini, Kimi, OpenCode, Aider, and Cursor

## Why PAD

- tmux-first: built around real panes and real workflows, not simulated tabs
- agent-aware: session preview, archive, hook updates, and provider-specific history are first-class
- fast to scan: the sidebar is optimized for "what changed" and "where do I go now"
- low-friction: keyboard-first, attach fast, detach fast, stay inside your terminal

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

Requires: `tmux` at runtime.

Supported runtime environments:

- macOS
- Linux
- WSL2

```bash
# One-line installer
curl -fsSL https://raw.githubusercontent.com/T1mn/pad/main/install.sh | bash

# Or from a local clone
git clone https://github.com/T1mn/pad.git
cd pad
./install.sh
```

The installer tries a pre-built release first, falls back to a source build if needed, and will offer to install `tmux` automatically when it is missing.

Manual source build:

```bash
cd pad/rust-tui
cargo build --profile dist
cp target/dist/pad ~/.local/bin/
```

PAD is tmux-first. Install and run `tmux` in the same environment as `pad`. On WSL2, install and run both inside WSL.

## Usage

```bash
pad              # Launch TUI
pad --help       # Show help
pad --version    # Show version
```

Release and platform notes:

- [Platform Support](docs/platform-support.md)
- [Release Checklist](docs/release-checklist.md)

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
