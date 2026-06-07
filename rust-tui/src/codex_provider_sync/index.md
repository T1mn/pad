# codex_provider_sync

- `backup.rs`：同步前的临时文件备份、失败恢复与清理。
- `rollout.rs`：Codex rollout JSONL session_meta provider 重写。
- `state_db.rs`：Codex `state_5.sqlite` provider 批量更新。
- `helpers.rs`：provider 同步测试的临时 Codex home、rollout 与 SQLite 夹具。
- `tests.rs`：rollout、SQLite 与 PAD 私有 Codex home 同步测试。
