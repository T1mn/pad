# scanner

- `process_snapshot.rs`：按本轮 pane pid 批量读取主/子进程命令，低分配解析并缓存，减少重复 `ps/pgrep` 和全进程表解析。
- `../scanner.rs`：扫描 tmux panes，日志只记录 agent 与总耗时；Git 信息用一次 porcelain status 解析，减少每轮子进程。
- `tests.rs`：扫描、Git 解析与默认忽略的本机耗时测试，用于验证 TUI 卡顿优化。
