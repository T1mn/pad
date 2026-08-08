# paths/paths_tests

- `../paths_tests.rs` 内联 `support`：共享临时 HOME helper。
- `../paths_tests.rs` 内联 `bridge_hooks`：Claude/Codex bridge 模板、Codex hooks feature key 与 TOML helper 测试。
- `../paths_tests.rs` 内联 `claude_paths`：Claude 自定义配置目录与空值回退测试。
- `../paths_tests.rs` 内联 `runtime_layout`：runtime layout、wrapper 安装与 PAD Codex profile hooks 测试。
- `../paths_tests.rs` 内联 `codex_home`：PAD 私有 Codex home 配置复制、隔离和旧 symlink 清理测试。
- `../paths_tests.rs` 内联 `prompts`：Codex prompt 组合、seed、版本状态与旧 prompt 迁移测试。
