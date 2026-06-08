# session_continuity/storage

- `ledger.rs`：continuity ledger 读取、原子保存与 session record upsert。
- `snapshot.rs`：按 session id 或 transcript path 查找 continuity snapshot。
- `diagnostic.rs`：诊断事件 JSONL 追加。
