# codex_state

- `../codex_state.rs`：Codex 状态 facade，外露归档、迁移、查询和状态模型入口。
- `query.rs` / `query/`：状态查询入口、SQLite 只读查询与 source JSON 解析。
- `cache.rs`：线程查询缓存。
- `archive.rs` / `archive/`：Codex 线程归档/恢复，移动 rollout 并同步 DB。
- `migration.rs` / `migration/`：兼容旧版 PAD 写入的 rollout 路径，把旧私有 home 路径规范回官方 `~/.codex`。
- `pathing.rs`：路径定位。
- `model.rs`：状态模型。
- `util.rs`：辅助函数。
- `tests.rs` / `tests/`：按 query、selection、archive、migration 分组的测试。
