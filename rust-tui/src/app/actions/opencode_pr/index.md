# app/actions/opencode_pr

- `../opencode_pr.rs` 内联 `command`：组装并启动 `tmux new-window` 中的 OpenCode PR 命令。
- `../opencode_pr.rs` 内联 `parse`：从剪贴板文本、`#123` 或 GitHub `/pull/123` URL 提取 PR 编号。
- `../opencode_pr.rs` 内联 `text`：OpenCode PR 动作 toast 文案。
