# telegram/callbacks

- `dispatch.rs` / `dispatch/`：Telegram callback query 入口，分发 help/use-pane/approval。
- `approval.rs`：Codex approval callback data、pending 查找、按钮提示消息。
- `approval_tests.rs`：Codex approval prompt 文本回归测试。
- `../callbacks.rs`：保持原 callback API 的 facade。
