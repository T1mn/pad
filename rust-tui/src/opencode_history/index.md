# opencode_history

- `mod.rs`：OpenCode 历史入口，读取官方 SQLite 数据库。
- `model.rs`：OpenCode session 展示模型。
- `query.rs`：session 查询入口、归档过滤与 thread 组装。
- `query/`：message 摘要抽取与 model 字段解析。
- `stats.rs`：读取并格式化 OpenCode session share/cost/token 元数据，兼容旧 schema。
- `query_tests.rs`：OpenCode SQLite 查询测试夹具。
- `archive.rs`：通过 OpenCode session 的 `time_archived` 字段归档/恢复。
- `util.rs` / `util/`：OpenCode 数据库路径发现、SQLite 打开与错误转换。
