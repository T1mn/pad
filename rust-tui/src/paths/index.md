# paths

- `hook_bridge.rs`：Claude/Codex hook bridge 模板安装与 Codex hook 配置。
- `codex_hooks.rs` / `codex_hooks/`：Codex hooks feature 开关、`hooks.json` 写入与版本兼容。
- `codex_wrapper.rs`：安装 `~/.pad/scripts/pad-codex`，固定使用 PAD 私有 Codex home、profile 与 relay auth。
- `codex_home.rs`：`~/.pad/codex-home` 私有 Codex 配置/auth/hooks 路径初始化，与官方 `~/.codex` 隔离。
- `prompts.rs` / `prompts/`：Codex jailbreak/index prompt 路径、种子刷新、组合 prompt 生成与版本状态。
- `sounds.rs`：声音资源目录和 preset WAV 文件路径。
- `runtime_files.rs`：hook/API socket、状态文件和 Telegram runtime 文件路径。
- `paths_tests.rs`：runtime layout、prompt seed、hook bridge 模板测试。
- `~/.pad/notifications/inbox.json`：notification inbox 持久化文件。
- `~/.pad/opencode-exports/` / `opencode-stats/` / `opencode-diagnostics/`：OpenCode 导出 JSON、stats 与诊断报告。

- `~/.pad/workspace-recipes.toml`：workspace recipe 配置文件。
- `~/.pad/pad-api.sock`：socket API 监听地址。
- `~/.pad/codex-turn-diffs/`：Codex 单轮问答 diff 的 pending、patch 与索引存储目录。
