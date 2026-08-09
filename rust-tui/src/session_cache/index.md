# session_cache

- `mod.rs`：缓存入口。
- `mod.rs`：缓存常量、持久化记录、hook binding 上下文和快照模型。
- `mod.rs` / `persist.rs` / `bindings.rs`：存储、hook/transcript 持久化与 Codex history snapshot 加载。
- `turns.rs` / `turns_tests.rs`：最近对话合并、裁剪与 Codex prompt 归一化规则。
- `mod.rs` 内联 `util`：缓存辅助函数。
- `tests.rs`：turn 合并与 hook 持久化回归测试。
