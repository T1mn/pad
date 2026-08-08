# actions

- `tree.rs` / `tree/`：tree 开关、agent launcher/fuzzy picker 与文件预览 helper。
- `thread_actions.rs` / `thread_actions/`：线程归档、恢复、回收站恢复的视图、目标筛选与确认执行。
- `thread_meta_edit.rs`：thread 标题与标签编辑保存。
- `thread_panel_delete.rs`：删除 native live pane、隐藏对应 thread 与更新本地列表。
- `codex_restart.rs` / `codex_restart/`：选中 Codex pane 原地强制重启，用 PAD Codex runtime resume。
- `../actions.rs` 内联 `panel_width`：调整并持久化左侧 agent 列表宽度。
- `native_launch.rs`：action 层 native agent terminal 启动入口，统一 pane 尺寸回退、terminal focus 与 sidebar registry 接入。
- `opencode_cli.rs`：统一保留 OpenCode 完整配置命令（路径展开、引号与附加参数），安全追加 action 参数，并让同步 action 通过明确的 shell argv 执行；回归测试集中在 `opencode_tests.rs`。
- `opencode_export.rs` / `opencode_export/`：OpenCode export/sanitized export 动作、CLI 调用、路径生成与 toast 文案。
- `opencode_github.rs` / `opencode_github/` / `opencode_import.rs` / `opencode_import/` / `opencode_plugin.rs` / `opencode_plugin/` / `opencode_pr.rs` / `opencode_pr/` / `opencode_run.rs` / `opencode_run/` / `opencode_serve.rs` / `opencode_serve/` / `opencode_stats.rs` / `opencode_stats/` / `opencode_diagnostics.rs` / `opencode_diagnostics/` / `opencode_attach.rs` / `opencode_attach/` / `opencode_web.rs` / `opencode_web/`：调用 OpenCode 官方 github/import/plugin/pr/run/serve/stats/attach/web 与只读诊断；交互式启动统一进入 native terminal 与左侧 registry。
- `notification_inbox.rs` / `notification_inbox/` / `notification_inbox_tests.rs`：notification inbox 打开、导航、标记已读、删除与持久化。
- `settings.rs` / `settings/`：设置项读写、列表项与状态同步。
- `relay_reload.rs` / `relay_reload/` / `relay_reload_helpers.rs` / `relay_reload_tests.rs` / `relay_reload_tests/`：relay 配置刷新、比较/应用、提示文案与外部变更回归测试。
- `helpers.rs` / `helpers/`：动作层设置搜索、thread meta 与 toast 辅助函数。
- `tests.rs` / `opencode_tests.rs` / `*_tests.rs`：动作层公共回归、集中 OpenCode 动作测试与少量深层模块测试。
