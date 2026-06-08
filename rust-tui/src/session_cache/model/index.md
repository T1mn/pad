# session_cache/model

- `../model.rs`：缓存模型 facade、版本/保留时间/turn 数限制常量。
- `records.rs`：持久化索引、session record 与 pane binding record。
- `context.rs`：从 hook event 提取 pane/session/path binding 上下文。
- `snapshot.rs`：对外快照模型与 record -> snapshot 转换。
- `support.rs`：判断 panel 是否支持 session cache，并包含支持矩阵测试。
