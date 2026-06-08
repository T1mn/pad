# codex_hooks

- `version.rs`：检测 Codex CLI 版本，并选择新旧 hooks feature key。
- `toml_edit.rs`：最小 TOML section 布尔值写入工具。

- `feature.rs`：按 Codex CLI 版本启用正确的 hooks feature key，并清理旧 key。
- `hooks_json.rs`：写入 `hooks.json` 中 PAD hook bridge 的 SessionStart/UserPromptSubmit/Stop 命令。
