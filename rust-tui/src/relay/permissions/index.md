# permissions

- `toml_helpers.rs` / `toml_helpers/`：Codex `config.toml` runtime overlay 的嵌套读写、恢复与空表清理。
- `json_helpers.rs` / `json_helpers/`：Claude `settings.json` runtime overlay 的嵌套读写、恢复与空对象清理。
- `codex.rs` / `codex/`：Codex runtime overlay 应用/恢复、字段写入拆分与原始状态保存。
- `claude.rs`：Claude permission overlay 应用/恢复与原始状态保存。
