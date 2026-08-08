# codex_state/tests

- `../tests.rs` 内联 `support`：共享临时 DB、Codex home、rollout 与 SQLite 夹具。
- `../tests.rs` 内联 `query`：状态 DB 读取、归档过滤与按 thread id 查询回归。
- `../tests.rs` 内联 `selection`：按 cwd 选择最新相关 Codex thread 的回归测试。
- `../tests.rs` 内联 `archive` / `../tests.rs` 内联 `archive_compressed`：归档/恢复 rollout、压缩 sibling 与 DB 同步测试。
- `../tests.rs` 内联 `migration`：旧 PAD 私有 Codex home rollout 路径迁移测试。
