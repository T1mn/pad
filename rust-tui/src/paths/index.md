# paths

- `hook_bridge.rs`：Claude/Codex hook bridge 模板安装与 Codex hook 配置。
- `codex_hooks.rs` / `codex_hooks/`：Codex hooks feature 开关、`hooks.json` 写入与版本兼容。
- `codex_home.rs`：pad 专用 Codex Home 初始化，session/db 共享链接。
- `paths_tests.rs`：runtime layout、prompt seed、hook bridge 模板测试。
- `~/.pad/notifications/inbox.json`：notification inbox 持久化文件。
- `~/.pad/opencode-exports/` / `opencode-stats/` / `opencode-diagnostics/`：OpenCode 导出 JSON、stats 与诊断报告。

- `~/.pad/workspace-recipes.toml`：workspace recipe 配置文件。
- `~/.pad/pad-api.sock`：socket API 监听地址。
- `~/.pad/codex-turn-diffs/`：Codex 单轮问答 diff 的 pending、patch 与索引存储目录。
