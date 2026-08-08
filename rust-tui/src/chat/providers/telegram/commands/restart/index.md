# restart

- `../restart.rs` 内联 `target`：选择 `/restart` 应该 respawn 的 tmux pane，或创建新的 detached session。
- `../restart.rs` 内联 `shell`：构建 rebuild + exec 当前 pad 的 shell 命令，并过滤 `telegram-bot` 子命令。
- `../restart.rs` 内联 `execute`：把 restart plan 交给 tmux 执行。
