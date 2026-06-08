# plain preview

- `../plain.rs`：纯文本预览主渲染入口，只克隆可见行窗口。
- `cache.rs`：按 target、宽度、主题和内容 revision 构建/复用纯文本渲染缓存。
- `scroll.rs`：基于缓存或内容精算 preview scroll 上限与 wrapped row 数。
- `window.rs` / `window_tests.rs`：按 scroll/viewport 计算需克隆的可见行窗口，避免长输出每帧全量复制。
