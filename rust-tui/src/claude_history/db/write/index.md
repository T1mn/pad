# claude_history/db/write

- `../write.rs` 内联 `scan`：扫描序号读取和扫描线程 row upsert。
- `../write.rs` 内联 `archive`：归档/恢复归档状态变更。
- `hook.rs`：Claude hook 上报 session 时的索引 upsert。
