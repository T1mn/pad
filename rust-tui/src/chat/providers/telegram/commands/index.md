`commands/` 负责 Telegram 文本命令入口。

- `update.rs`：接收 Telegram update，区分 slash/plain text/callback。
- `command.rs`：slash 命令分发。
- `diag.rs`：`/diag` 和 `/padstatus`。
- `history.rs`：`/history` 历史摘要。
- `restart.rs`：`/restart` 目标选择与 shell 命令生成。
- `slash.rs`：向 Codex pane 发送 `/status`、`/fast`、`/compact` 并轮询输出。
- `plain.rs`：普通文本 prompt 投递与 pending 建档。
- `help_actions.rs`：帮助页与 agent 列表消息。
