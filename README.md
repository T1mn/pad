<div align="center">
  <h1>PAD</h1>
  <p><strong>One workspace for multiple AI agents in tmux.</strong></p>
  <p><code>pad</code> = Panel for Agent Development.</p>
  <p>English | <a href="README_ZH.md">中文</a></p>
</div>

PAD gives you one place to manage Codex, Claude Code, Gemini, and other terminal agents.
You can see which session moved, read recent conversation history, and only then jump into the right pane.

## TL;DR

- Manage multiple AI agent sessions from one workspace.
- Read recent conversation history before you attach to a pane.
- Pure Rust. Tmux-native. Built for terminal agents.
- Current macOS dist build: ~3.7 MB. Idle runtime: ~12 MB RSS.

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

The installer tries a pre-built release first, validates that the downloaded binary can run on the current machine, and falls back to a local source build when needed. It also installs `tmux` automatically when missing, and will bootstrap Rust plus common build tools when a source build is required.

Manual source build:

```bash
cd pad/rust-tui
cargo build --profile dist
cp target/dist/pad ~/.local/bin/
```

PAD is tmux-first. Install and run `tmux` in the same environment as `pad`. On WSL2, install and run both inside WSL.

## Demo

<video src="https://github.com/user-attachments/assets/773baf57-c25f-41d4-a30a-3c38e702d2d8" controls muted loop playsinline width="960"></video>

What this looks like in practice:

- Manage multiple agents in one workspace instead of hunting across tmux panes
- Read the latest preview and recent turns on the right before you attach
- Hit `Tab` to open the latest detail view and `Shift+J` / `Shift+K` to move across Q&A turns
- Create a fresh session with `c`, send work, then jump back to PAD with `F12`
- Use the activity indicator to see which session is still running in the background

If your Markdown viewer does not render inline video, open the [demo video](https://github.com/user-attachments/assets/773baf57-c25f-41d4-a30a-3c38e702d2d8) directly.

## Why PAD

The usual tmux workflow breaks down in a very boring way:

- I have Codex, Claude Code, and Gemini open. Which one actually moved?
- Which pane moved last?
- Which session is still working?
- Do I need to attach, or is the answer already visible in recent history?
- If I archive this thread, am I hiding it or actually deleting something?

PAD gives you one workspace to scan, preview, attach, archive, and jump back out without losing your place.

## 30-Second Workflow

1. Run `pad`.
2. Scan the left sidebar for the session that moved.
3. Read the latest turns in preview before you attach.
4. Hit `Enter` to jump in, then `F12` or `Ctrl+Q` to come back.

## Core Features

- One workspace view for live Codex, Claude Code, Gemini, and other agent sessions
- See recent session history and latest turns without entering the pane
- Pure Rust TUI with a small footprint and quick session-aware previews
- Current macOS measurement: ~3.7 MB dist binary, ~12 MB idle RSS
- Session-level monitoring so activity tracking stays focused and cheap
- Jump into a pane with `Enter`, return with `F12` or `Ctrl+Q`
- Archive threads without touching upstream session data
- Relay / proxy settings for supported agents
- Completion notifications when an agent finishes, on supported desktop backends
- Telegram bot daemon for remote updates and quick session access
- Keyboard-first search, settings, tree, and session creation

## What PAD Does Not Do

- It does not replace tmux.
- It does not fake tabs on top of tmux panes.
- It does not delete upstream agent history when you archive a thread in PAD.
- It does not take over the agent runtime. It helps you see and jump faster.

## Screen Tour

### Overview

<img src="docs/media/first-annotated.png" alt="PAD home screen overview" width="960">

Open PAD here first. This is the fast scan view.

1. `LIVE 6`: the top-level live inbox and current online session count.
2. Highlighted session row: the current target in the sidebar, ready for preview or attach.
3. Preview header: agent, state, PID, branch, path, and SID at a glance.
4. Preview turns: read the latest Q/A before you decide to attach.

### Settings

<img src="docs/media/settings-annotated.png" alt="PAD settings overview" width="960">

Settings stays in flow. Open it with `/`, change what you need, leave with `Esc`.

1. `/` prompt: settings comes from the same slash-driven flow as other terminal-first tools.
2. Settings list: move through config areas without leaving the keyboard.
3. Inline current values: scan current state directly from the list.
4. Footer hints: the active keys are always visible at the bottom.

### Archive

<img src="docs/media/archive-annotated.png" alt="PAD archive confirmation overview" width="960">

Archive in PAD is narrow on purpose. It matches the Codex-side mental model: hide it from PAD, keep the original session data intact.

1. Confirmation dialog: archive is explicit and reversible. It is not delete.
2. Target thread: the dialog shows exactly which thread is being archived before you confirm.
3. Live pane warning: if the thread still has a live pane, PAD tells you clearly that archive only hides it in PAD and updates PAD's local index.
4. Codex-aligned semantics: PAD keeps upstream session data untouched and only updates its own tracking layer. For Claude that means PAD updates its Claude sqlite index and does not modify the original `~/.claude` session source.

### Tree

<img src="docs/media/tree-annotated.png" alt="PAD tree view overview" width="960">

Use tree mode when you want to browse code, preview a file, or create a session from a directory without leaving PAD.

1. Root path: the current workspace is always visible at the top.
2. File tree: expand, collapse, and move through directories quickly.
3. File preview: inspect code immediately on the right.
4. Tree footer: tree-mode keys stay visible, including nav, expand, attach, create, and help.

### Help

<img src="docs/media/help-annotated.png" alt="PAD help overview" width="960">

Help keeps the keyboard model discoverable inside the UI, so you do not have to context-switch to docs.

1. Help header: you are looking at PAD's built-in keyboard guide, not an external doc.
2. Navigation section: movement, jump, and search keys are grouped together.
3. Actions section: attach, create, delete, refresh, focus switching, and preview controls live in one place.
4. Close hint: the footer shows the shortest way back out.

## Also Included

- Git context in the preview header: branch, commit, and changed files
- Busy / waiting state indicators for live agent panes
- File tree browsing with file preview
- Theme switching
- Agent launcher from the tree view

## Usage

```bash
pad              # Launch TUI
pad --help       # Show help
pad --version    # Show version
pad telegram-bot # Launch Telegram bot daemon
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
| `d` | Delete pane and hide thread in PAD |
| `r` | Refresh |
| `Ctrl+F` | Search panels |
| `/` | Open settings |
| `F1` | Settings |
| `q` | Quit |

## Agent Support

Fully supported session workflows:

- 🟣 Claude (`claude`)
- 🔵 Codex (`codex`)
- 🔷 Gemini (`gemini-cli`)

Basic launcher / pane workflows:

- 🟢 Kimi (`kimi-cli`)
- 🟠 OpenCode (`opencode`)

PAD can still detect and attach to other terminal agents, but history, archive, and session-aware preview are currently centered on Codex, Claude, and Gemini.

## Acknowledgements

Thanks to the broader terminal tooling community for early feedback and testing. I also learned a lot of practical, project-helpful ideas from [linux.do](https://linux.do) along the way.

## License

MIT
