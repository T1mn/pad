# restart

- `target.rs`：选择 `/restart` 应该 respawn 的 tmux pane，或创建新的 detached session。
- `shell.rs`：构建 rebuild + exec 当前 pad 的 shell 命令，并过滤 `telegram-bot` 子命令。
- `execute.rs`：把 restart plan 交给 tmux 执行。
