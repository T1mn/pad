# tmux_dispatch/prompt

- `../prompt.rs` 内联 `literal`：单行 prompt 通过 `tmux send-keys -l` 分块发送。
- `../prompt.rs` 内联 `paste`：多行或 literal 失败时通过 tmux paste buffer 发送。
- `util.rs` / `util_tests.rs`：多行判断、literal 分块、提交延迟与 buffer 时间戳辅助。
