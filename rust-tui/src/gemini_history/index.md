# gemini_history

- `mod.rs`：Gemini 历史入口。
- `scan.rs` / `scan/`：历史扫描聚合、session 文件遍历与 snapshot 解析。
- `storage.rs`：持久化 facade，保持查询/写入调用路径稳定。
- `storage/`：schema/open、查询、写入/归档实现。
- `model.rs`：数据模型。
- `util.rs` / `util/`：路径、文件、哈希、mtime 与 timestamp 解析辅助。
- `tests.rs`：测试。
