# apply

- `state.rs`：把 `PreviewUpdate` 应用到 preview 主状态，处理 session/plain 视图切换和滚动保留。
- `snapshot.rs`：记录应用前状态，并统一判断是否需要置 dirty。
- `thread_cache.rs`：更新 thread preview cache，并在 mtime 或裁剪变化时刷新 sidebar cache。
- `panel.rs`：把 session transcript、turns 和 session id 回写到 live panel。
