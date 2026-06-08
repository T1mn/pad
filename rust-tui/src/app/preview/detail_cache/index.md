# app/preview/detail_cache

- `request.rs`：从当前 preview 状态构造 detail render request。
- `lookup.rs`：当前 cache 与 LRU cache 查询、命中后提升。
- `store.rs`：写入 detail cache 并维护 LRU 上限。
- `matchers.rs`：按 turns allocation 或 request 字段判断 cache 命中。
