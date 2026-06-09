# attach/tmux

- `command.rs` / `command_tests.rs`：tmux 命令执行、成功判断与 stdout/stderr 调试摘要。
- `query.rs`：当前 session/window/pane 与目标 pane snapshot 查询。
- `shell.rs`：tmux run-shell 需要的 shell quoting、日志命令和 zoom 等待脚本。
- `status.rs`：tmux status 查询和 show/hide 切换。
