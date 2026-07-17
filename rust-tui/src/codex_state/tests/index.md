# codex_state/tests

- `support.rs`：共享临时 DB、Codex home、rollout 与 SQLite 夹具。
- `query.rs`：状态 DB 读取、归档过滤与按 thread id 查询回归。
- `selection.rs`：按 cwd 选择最新相关 Codex thread 的回归测试。
- `archive.rs` / `archive_compressed.rs`：归档/恢复 rollout、压缩 sibling 与 DB 同步测试。
- `migration.rs`：旧 PAD 私有 Codex home rollout 路径迁移测试。
