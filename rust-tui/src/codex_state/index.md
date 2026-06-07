# codex_state

- `query.rs`：状态查询入口。
- `cache.rs` / `archive.rs`：缓存与归档处理。
- `migration.rs`：兼容旧版 PAD 写入的 rollout 路径，把旧私有 home 路径规范回官方 `~/.codex`。
- `pathing.rs`：路径定位。
- `model.rs`：状态模型。
- `util.rs`：辅助函数。
- `tests.rs`：测试。
