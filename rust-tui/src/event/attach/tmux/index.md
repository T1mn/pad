# attach/tmux

- `command.rs` / `tests.rs` 内联 `command`：tmux 命令执行、成功判断与 stdout/stderr 调试摘要。
- `query.rs` / `tests.rs` 内联 `query`：当前 session/window/pane、目标 snapshot 与可写 client 查询。
- `shell.rs` / `tests.rs` 内联 `shell`：tmux run-shell 需要的 shell quoting、日志命令和 zoom 等待脚本。
- `../tmux.rs` 内联 `status`：tmux status 查询和 show/hide 切换。
