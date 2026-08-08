# restart

- `../restart.rs` 内联 `target`：记录 `/restart` 使用的启动目录与 shell 命令。
- `../restart.rs` 内联 `shell`：构建 rebuild + exec 当前 pad 的 shell 命令，并过滤 `telegram-bot` 子命令。
- `../restart.rs` 内联 `execute`：构建完成后启动新的 PAD 进程。
