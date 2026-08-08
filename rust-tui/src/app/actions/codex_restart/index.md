# codex_restart

- `../codex_restart.rs`：选中 Codex pane 原地重启入口，串起 preflight、runtime 准备与 native respawn。
- `../codex_restart.rs` 内联 `command`：生成 `pad-codex ... resume` shell 命令与 quoting。
- `../codex_restart.rs` 内联 `preflight`：只保留面板类型检查；Codex 的 Busy / Waiting / Idle 都允许重启。
- `../codex_restart.rs` 内联 `messages`：重启 toast 文案与 CJK 语言判断。
- `tests.rs`：重启 preflight 与命令构造回归测试。
