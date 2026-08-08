# paths

- `base.rs` / `base_tests.rs`：`PAD_HOME`（未设置时为 `~/.pad`）下基础目录、日志、脚本、session、配置等路径函数和基础路径测试。
- `../paths.rs` 内联 `claude`：统一解析 `CLAUDE_CONFIG_DIR`，并提供 Claude settings / projects 路径。
- `hook_bridge.rs` / `hook_bridge/`：Claude/Codex hook bridge 模板安装、状态检查与模板生成。
- `codex_hooks.rs` / `codex_hooks/`：Codex hooks feature 开关、`hooks.json` 写入与版本兼容。
- `codex_wrapper.rs` / `codex_wrapper_tests.rs`：安装 `~/.pad/scripts/pad-codex`，固定使用 PAD 私有 Codex home、profile 与 relay auth。
- `codex_home.rs`：`~/.pad/codex-home` 私有 Codex 配置/auth/hooks 路径初始化，与官方 `~/.codex` 隔离。
- `prompts.rs` / `prompts/`：Codex jailbreak/index prompt 路径、种子刷新、组合 prompt 生成与版本状态。
- `../paths.rs` 内联 `sounds`：声音资源目录和 preset WAV 文件路径。
- `../paths.rs` 内联 `runtime_files`：hook/API socket、状态文件和 Telegram runtime 文件路径。
- `paths_tests.rs` / `paths_tests/`：按 bridge hooks、runtime layout、Codex home、prompts 分组的路径测试。
- `~/.pad/notifications/inbox.json`：notification inbox 持久化文件。
- `~/.pad/opencode-exports/` / `~/.pad/opencode-stats/` / `~/.pad/opencode-diagnostics/`：OpenCode 导出 JSON、stats 与诊断报告。

- `~/.pad/workspace-recipes.toml`：workspace recipe 配置文件。
- `~/.pad/terminal-workspace.json`：原生终端的 tab、split、label 与启动 profile 工作区快照；不保存 PTY 进程内容；无效文件隔离为同目录 `terminal-workspace.invalid*.json`。
- `~/.pad/pad-api.sock`：socket API 监听地址。
- `~/.pad/codex-turn-diffs/`：Codex 单轮问答 diff 的 pending、patch 与索引存储目录。
