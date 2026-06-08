# opencode_history/query

- `db.rs`：执行 OpenCode SQLite session 查询、表存在性检查与单 session 查找。
- `thread.rs`：把 session 行和消息摘要组装成 `OpenCodeThreadRef`。
- `messages.rs`：从 message/part 表读取首尾用户消息和最后助手消息摘要。
- `model_parse.rs`：解析 OpenCode session 的 provider/model JSON 字段。
