# app/preview/preview_tests

- `latest.rs`：打开最新预览 turn 的选择逻辑测试。
- `cache_dirty.rs` / `cache_dirty/`：预览更新、detail cache、dirty 标记测试。
- `selection_scroll.rs` / `selection_scroll/`：detail selection、plain follow-bottom、panel cache 与 context reset 测试。
- `tick_cache.rs` / `tick_cache/`：plain cache、debounce/detail、busy tick 与 thread cache 裁剪测试。
