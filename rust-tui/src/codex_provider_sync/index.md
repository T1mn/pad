# codex_provider_sync

- `backup.rs`：同步前的临时文件备份、失败恢复与清理。
- `model.rs`：provider 同步结果模型。
- `sync.rs`：一次 provider 同步事务，包含 rollout/SQLite 更新与失败回滚。
- `worker.rs`：后台 provider 同步队列，只保留最后一个待同步 provider。
- `rollout.rs` / `rollout/`：Codex rollout JSONL 扫描、首行 provider 重写与安全写回。
- `state_db.rs`：Codex `state_5.sqlite` provider 批量更新。
- `helpers.rs`：provider 同步测试的临时 Codex home、rollout 与 SQLite 夹具。
- `tests.rs`：rollout、SQLite 与 PAD 私有 Codex home 同步测试。
