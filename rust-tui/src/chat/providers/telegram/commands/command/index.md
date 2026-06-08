# telegram/commands/command

- `use_target.rs`：`/use` 编号解析、live pane stale 校验与目标切换。
- `restart_cmd.rs`：`/restart` plan 获取、提示与失败反馈。
- `stop.rs`：`/stop` 向当前目标 pane 发送 Escape。
- `reset.rs`：`/reset` 清理当前目标 pending 并回传状态。
