# ui/preview/session_list_cache

- `../session_list_cache.rs` 内联 `build`：构建/复用 session list card 缓存，并记录慢重建日志。
- `../session_list_cache.rs` 内联 `matchers`：判断缓存是否匹配当前 target、宽度、主题和 turns allocation。
- `../session_list_cache.rs` 内联 `range`：根据选中 turn 计算列表行范围。
- `../session_list_cache.rs` 内联 `visible`：从缓存中按 scroll/height 取可见行，按需生成 gap 行。
- `tests.rs`：session list cache 回归测试。
