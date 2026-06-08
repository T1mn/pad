# session_cache

- `mod.rs`：缓存入口。
- `model.rs` / `model/`：缓存常量、持久化记录、hook binding 上下文和快照模型。
- `storage.rs` / `persist.rs` / `persist/` / `bindings.rs` / `bindings/`：存储、持久化与 pane/session 绑定查找写入。
- `turns.rs` / `turns/` / `turns_tests.rs`：最近对话合并、裁剪与 Codex prompt 归一化规则。
- `preload.rs`：启动/扫描预热；已具备 session 信息的 panel 会跳过索引读取。
- `util.rs`：缓存辅助函数。
- `tests.rs` / `tests/`：按 turns、bindings、preload 分组的缓存测试。

- `list_cached_sessions()`：为 agent resume/socket API 暴露缓存会话摘要。
