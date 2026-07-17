# grok_history

- `mod.rs`：Grok Build 历史入口，遵循 `GROK_HOME`。
- `scan.rs`：扫描官方 `sessions/*/*/summary.json`，单个损坏会话安全跳过。
- `model.rs`：历史线程模型。
- `tests.rs`：官方 0.2.102 格式与未知字段回归测试。
