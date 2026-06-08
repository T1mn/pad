# chat/providers/telegram/hooks/tests

- `support.rs`：共享 pending request 与 stop hook event 夹具。
- `completion.rs`：Codex stop 结果优先从 transcript completion 解析的回归测试。
- `turn_match.rs`：pending turn 与 stop turn 匹配/缺失保护测试。
- `phase_gate.rs`：pending 尚未 submit 时忽略 stop 的阶段保护测试。
