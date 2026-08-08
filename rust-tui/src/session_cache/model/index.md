# session_cache/model

- `../model.rs`：缓存模型 facade、版本/保留时间/turn 数限制常量。
- `../model.rs` 内联 `records`：持久化索引、session record 与 pane binding record。
- `../model.rs` 内联 `context`：从 hook event 提取 pane/session/path binding 上下文。
- `../model.rs` 内联 `snapshot`：对外快照模型与 record -> snapshot 转换。
- `support.rs` / `support_tests.rs`：判断 panel 是否支持 session cache，并包含支持矩阵测试。
