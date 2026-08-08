# session_cache/bindings/lookup

- `../lookup.rs` 内联 `matching`：判断 pane binding 是否精确命中或 fallback 命中当前 panel。
- `../lookup.rs` 内联 `unique`：从候选 session id 中筛出唯一命中，并跳过 Codex subagent session。
- `../lookup.rs` 内联 `snapshot`：按 session id 或 agent type 组装 `SessionCacheSnapshot`。
