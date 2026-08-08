# app/preview/detail_cache

- `../detail_cache.rs` 内联 `request`：从当前 preview 状态构造 detail render request。
- `lookup.rs`：当前 cache 与 LRU cache 查询、命中后提升。
- `../detail_cache.rs` 内联 `store`：写入 detail cache 并维护 LRU 上限。
- `../detail_cache.rs` 内联 `matchers`：按 turns allocation 或 request 字段判断 cache 命中。
