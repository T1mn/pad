# grok_history

- `mod.rs`：Grok Build 历史入口，遵循 `GROK_HOME`。
- `mod.rs`：扫描官方 `sessions/*/*/summary.json`，单个损坏会话安全跳过。
- `mod.rs` 内联 `model`：历史线程模型。
- `mod.rs`：官方 0.2.102 格式与未知字段回归测试。
