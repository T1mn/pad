`commands/` 负责 Telegram 文本命令入口。

- `update.rs`：接收 Telegram update，区分 slash/plain text/callback。
- `command.rs` / `command/`：slash 命令分发与 use/restart/stop/reset 子命令。
- `diag.rs` / `diag/`：`/diag` 上下文解析、诊断文本和 `/padstatus`。
- `history.rs`：`/history` 历史摘要。
- `restart.rs` / `restart/`：`/restart` 目标选择、shell 命令生成与 tmux 执行。
- `slash.rs` / `slash/`：向 Codex pane 发送 `/status`、`/fast`、`/compact`，拆分目标校验与输出轮询。
- `plain.rs`：普通文本 prompt 投递与 pending 建档。
- `help_actions.rs`：帮助页与 agent 列表消息。
