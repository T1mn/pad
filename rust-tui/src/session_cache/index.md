# session_cache

- `mod.rs`：缓存入口。
- `model.rs`：缓存数据模型。
- `storage.rs` / `persist.rs` / `bindings.rs`：存储、持久化与绑定。
- `preload.rs`：启动/扫描预热；已具备 session 信息的 panel 会跳过索引读取。
- `util.rs`：缓存辅助函数。
- `tests.rs`：缓存测试。

- `list_cached_sessions()`：为 agent resume/socket API 暴露缓存会话摘要。
