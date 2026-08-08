# session_continuity/storage

- `../storage.rs` 内联 `ledger`：continuity ledger 读取、原子保存与 session record upsert。
- `../storage.rs` 内联 `snapshot`：按 session id 或 transcript path 查找 continuity snapshot。
- `../storage.rs` 内联 `diagnostic`：诊断事件 JSONL 追加。
