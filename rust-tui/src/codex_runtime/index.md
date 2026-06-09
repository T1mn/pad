# codex_runtime

- `command.rs`：生成 `pad-codex` wrapper 命令、Claude 启动时清理继承的 Anthropic env、移除用户传入 profile、shell quoting 与命令识别。
- `auth.rs`：检查 PAD Codex profile 是否需要 OpenAI auth，并读取 `auth.json` / 环境变量。
- `tests.rs`：wrapper 命令、profile 移除与 relay auth 校验回归。
