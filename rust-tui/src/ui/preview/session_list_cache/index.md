# ui/preview/session_list_cache

- `build.rs`：构建/复用 session list card 缓存，并记录慢重建日志。
- `matchers.rs`：判断缓存是否匹配当前 target、宽度、主题和 turns allocation。
- `range.rs`：根据选中 turn 计算列表行范围。
- `visible.rs`：从缓存中按 scroll/height 取可见行，按需生成 gap 行。
- `tests.rs`：session list cache 回归测试。
