# PAD

Stop hunting panes. Run your AI workflow from one place.

`pad` is the CLI for PAD: Panel for Agent Development.

PAD is a tmux-native control panel for Codex, Claude Code, Gemini CLI, Kimi, OpenCode, Aider, Cursor, and other terminal AI agents.

PAD is built for the moment when you have more than one agent, more than one session, and you need a fast way to see what is running, what just moved, and where to jump next.

## Demo

<video src="docs/media/new_waiting.mp4" controls muted loop playsinline width="960"></video>

This flow shows the keyboard-native loop PAD is optimized for:

- Open PAD and create a fresh session with `c`
- Send work, return to the dashboard with `F12`, and keep the session running
- Read the breathing activity indicator to see that the agent is still working in the background
- Double-tap `Tab` to jump from the session list into the latest preview detail
- Use `Shift+J` / `Shift+K` to move across Q&A turns inside preview detail before you attach again

If your Markdown viewer does not render inline video, open [`new_waiting.mp4`](docs/media/new_waiting.mp4) directly.

## Core Features

- Unified agent inbox for live panes and historical sessions
- Preview recent turns before you attach to a pane
- Drill into preview detail without leaving the keyboard
- Jump in fast and return safely with tmux-native handoff
- Keep archive state, session metadata, and relay settings in one place

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
curl -fsSL https://raw.githubusercontent.com/T1mn/pad/master/install.sh | bash

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
| `J/K` or `Shift+J/K` | Move between preview turns / jump faster in preview |
| `1-9` | Jump to panel |
| `Enter` | Attach to panel |
| `F12` / `Ctrl+Q` | Detach back to pad |
| `Tab` | Toggle panel focus and preview focus |
| `Tab` twice | Open the latest preview detail, or return detail back to the turns list |
| `?` | Help |
| `t` | Toggle file tree |
| `T` | Open tree at ~/ |
| `Space` | Expand/collapse directory |
| `Space` twice | Expand/collapse all session folders |
| `c` | Create new session |
| `d` | Delete panel |
| `r` | Refresh |
| `Ctrl+F` | Search panels |
| `/` | Open settings |
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
