# scanner

- `process_snapshot.rs`：一次性读取 `ps` 进程表，缓存 pane 主进程和直接子进程命令，减少扫描时重复 `ps/pgrep`。
- `../scanner.rs`：扫描 tmux panes，日志只记录 agent 与总耗时，避免每轮输出全部非 agent pane。
