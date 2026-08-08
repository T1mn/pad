# app/hooks/hooks_tests

- `../hooks_tests.rs` 内联 `support`：共享临时 HOME、Codex state DB 与 hook event 构造器。
- `../hooks_tests.rs` 内联 `unread`：pane stop 未读标记与聚焦清理测试。
- `../hooks_tests.rs` 内联 `activity`：app thread activity 裁剪与 sidebar 排序不自动变更测试。
- `../hooks_tests.rs` 内联 `notification`：完成通知文案、声音事件与 notification inbox 测试。
- `../hooks_tests.rs` 内联 `session_cache`：pane hook 切换 session 时的缓存隔离回归测试。
