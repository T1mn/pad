# chat/backend

- `panels.rs`：live panel 扫描、session cache preload 与短 TTL 缓存。
- `text.rs`：slash command、pane capture 摘要、panel 显示标题和目标 label。
- `../backend.rs` 内联 `status`：读取 PAD runtime status 判断在线状态。
- `tests.rs`：panel 标题 fallback 与 thread meta override 回归测试。
