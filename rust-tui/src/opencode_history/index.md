# opencode_history

- `mod.rs`：OpenCode 历史入口，读取官方 SQLite 数据库。
- `model.rs`：OpenCode session 展示模型。
- `query.rs`：session/message/part 查询与摘要抽取。
- `stats.rs`：读取并格式化 OpenCode session share/cost/token 元数据，兼容旧 schema。
- `query_tests.rs`：OpenCode SQLite 查询测试夹具。
- `archive.rs`：通过 OpenCode session 的 `time_archived` 字段归档/恢复。
- `util.rs`：默认数据路径、`opencode db path` 兜底与 SQLite 错误转换。
