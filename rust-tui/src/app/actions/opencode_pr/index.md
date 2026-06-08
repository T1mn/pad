# app/actions/opencode_pr

- `command.rs`：组装并启动 `tmux new-window` 中的 OpenCode PR 命令。
- `parse.rs`：从剪贴板文本、`#123` 或 GitHub `/pull/123` URL 提取 PR 编号。
- `text.rs`：OpenCode PR 动作 toast 文案。
