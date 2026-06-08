# codex_state/query

- `db.rs`：只读打开 Codex `state_5.sqlite`，按归档状态或 thread id 映射 `CodexThreadRef`。
- `source.rs`：从 Codex thread `source` JSON 中解析 subagent parent thread id。
