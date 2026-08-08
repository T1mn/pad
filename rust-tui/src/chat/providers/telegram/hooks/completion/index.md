# hooks/completion

- `resolve.rs`：pending stop 结果来源选择，含 Codex transcript catch-up 重试。
- `../completion.rs` 内联 `cache`：把解析到的完成结果缓存回 pending request。
- `../completion.rs` 内联 `log`：完成/延迟投递日志。
- `model.rs`：完成结果与投递 outcome 模型。
