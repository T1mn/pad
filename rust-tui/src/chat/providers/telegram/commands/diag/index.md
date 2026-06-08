# diag

- `context.rs`：解析 `/diag` 的 request id、pane id、session/transcript 参数到诊断上下文。
- `format.rs`：把诊断上下文格式化成 Telegram 文本。
- `format_tests.rs`：`/diag` 空诊断上下文文本回归测试。
- `status.rs`：构建 `/padstatus` 消息体。
