# session_cache/bindings/lookup

- `matching.rs`：判断 pane binding 是否精确命中或 fallback 命中当前 panel。
- `unique.rs`：从候选 session id 中筛出唯一命中，并跳过 Codex subagent session。
- `snapshot.rs`：按 session id 或 agent type 组装 `SessionCacheSnapshot`。
