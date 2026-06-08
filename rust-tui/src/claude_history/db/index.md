# claude_history/db

- `schema.rs`：Claude 索引库建表、迁移与打开连接。
- `query.rs`：按状态或 session id 查询索引库并映射展示模型。
- `write.rs` / `write/`：扫描序号、索引 upsert、归档状态变更与 Hook upsert。
