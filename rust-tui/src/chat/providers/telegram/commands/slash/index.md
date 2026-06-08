# telegram/commands/slash

- `target.rs`：校验当前 Telegram 选中目标是否为可接收 slash 的 Codex pane。
- `poll.rs`：发送 slash 后轮询 tmux pane tail，并过滤 echo-only 输出。
