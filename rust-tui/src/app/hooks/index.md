# app/hooks

- `activity.rs` / `app_thread.rs`：从 hook 推断 app thread 活跃状态、裁剪覆盖表，并清理已读 stop 标记。
- `notification.rs` / `notification_text.rs`：生成完成通知、收件箱草稿、桌面通知与提示音。
- `title_summary.rs`：Codex stop 后生成标题摘要请求。
- `claude_history.rs`：Claude hook session upsert 参数整理。
- `hooks_tests.rs`：hook 事件、线程活跃度、完成通知测试。
