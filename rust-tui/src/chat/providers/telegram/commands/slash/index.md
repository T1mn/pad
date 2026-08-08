# telegram/commands/slash

- `../slash.rs` 内联 `target`：校验当前 Telegram 选中目标是否为可接收 slash 的 Codex pane。
- `../slash.rs` 内联 `poll`：发送 slash 后轮询 native pane frame，并过滤 echo-only 输出。
