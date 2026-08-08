# app/hooks

- `pane.rs` / `pane/`：有 native pane 的 hook 事件入口，拆分 panel 更新、subagent 过滤与后置副作用。
- `activity.rs` / `app_thread.rs`：从 hook 推断 app thread 活跃状态、裁剪覆盖表，并清理已读 stop 标记。
- `notification.rs` / `notification/` / `notification_text.rs`：生成完成通知、收件箱草稿、桌面通知与提示音。
- `../hooks.rs` 内联 `title_summary`：Codex stop 后生成标题摘要请求。
- `../hooks.rs` 内联 `claude_history`：Claude hook session upsert 参数整理。
- `hooks_tests.rs` / `hooks_tests/`：按 unread、activity、notification、session cache 分组的 hook 回归测试。
