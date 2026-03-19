# Tmux AI Kanban

Manage and monitor AI coding assistants (Claude, Codex, Kimi) running in tmux panes.

## Features

- 🔍 **Auto-detect** AI panels in tmux sessions
- 📊 **Table view** with rich formatting
- 🌿 **Git integration** - show branch, commit, status
- ⚡ **Activity indicators** - see which panels are active
- 🎯 **Filter** by AI type

## Installation

```bash
pip install -e .
```

## Usage

```bash
# List all AI panels
ai-kanban list

# Watch mode (auto-refresh)
ai-kanban list --watch

# Filter by AI type
ai-kanban list --filter claude

# Show summary
ai-kanban summary

# Jump to a pane
ai-kanban jump <session> <window> <pane>
```

## Supported AI Tools

- 🟣 Claude (claude)
- 🔵 OpenAI Codex (codex)
- 🟢 Kimi (kimi)
