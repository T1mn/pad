# gemini_history/storage

- `schema.rs`：Gemini 索引库建表、迁移与打开连接。
- `query.rs`：按归档状态、session id、cwd 查询索引并映射展示模型。
- `write.rs`：重建/更新索引记录与批量归档状态变更。
