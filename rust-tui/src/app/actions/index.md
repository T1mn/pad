# actions

- `tree.rs`：tree 开关、搜索与侧栏显示动作。
- `thread_actions.rs`：线程/会话级动作封装。
- `codex_restart.rs`：选中 Codex pane 原地重启、注入 `CODEX_HOME` 并 resume。
- `opencode_cli.rs`：OpenCode CLI 命令定位。
- `opencode_export.rs` / `opencode_import.rs` / `opencode_stats.rs`：调用 OpenCode 官方 export/import/stats。
- `notification_inbox.rs`：notification inbox 打开、导航、标记已读与删除。
- `settings.rs`：设置项读写与状态同步。
- `relay_reload.rs`：relay 配置刷新。
- `helpers.rs`：动作层公共辅助函数。
- `tests.rs` / `*_tests.rs`：动作层回归测试。
