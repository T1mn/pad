# opencode_history

- `mod.rs` / `mod_tests.rs`：OpenCode 历史入口，读取官方 SQLite 数据库。
- `mod.rs` 内联 `model`：OpenCode session 展示模型。
- `query.rs`：session 查询入口、归档过滤与 thread 组装。
- `query.rs`：message 摘要抽取与 model 字段解析。
- `stats.rs` / `mod_tests.rs` 内联 `stats`：读取并格式化 OpenCode session share/cost/token 元数据，兼容旧 schema。
- `mod_tests.rs` 内联 `query`：OpenCode SQLite 查询测试夹具。
- `mod.rs` / `mod_tests.rs` 内联 `archive`：只更新 `time_archived`，不改变会话排序；写连接等待短暂锁。
- `util.rs` / `util/`：OpenCode 数据库路径发现、SQLite 打开与错误转换。
