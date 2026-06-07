# actions

- `tree.rs`：tree 开关、搜索与侧栏显示动作。
- `thread_actions.rs`：线程/会话级动作封装。
- `codex_restart.rs`：选中 Codex pane 原地强制重启，用 `--profile pad` resume。
- `opencode_cli.rs`：OpenCode CLI 命令定位。
- `opencode_export.rs` / `opencode_github.rs` / `opencode_import.rs` / `opencode_plugin.rs` / `opencode_pr.rs` / `opencode_run.rs` / `opencode_serve.rs` / `opencode_stats.rs` / `opencode_diagnostics.rs` / `opencode_attach.rs` / `opencode_web.rs`：调用 OpenCode 官方 export/github/import/plugin/pr/run/serve/stats/attach/web 与只读诊断。
- `notification_inbox.rs`：notification inbox 打开、导航、标记已读与删除。
- `settings.rs`：设置项读写与状态同步。
- `relay_reload.rs` / `relay_reload_helpers.rs` / `relay_reload_tests.rs`：relay 配置刷新、比较/提示文案与外部变更回归测试。
- `helpers.rs`：动作层公共辅助函数。
- `tests.rs` / `*_tests.rs`：动作层回归测试。
