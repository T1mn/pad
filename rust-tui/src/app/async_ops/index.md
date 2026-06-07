# app/async_ops

- `scan.rs`：tmux pane 扫描调度、结果合并和延迟扫描。
- `preview_update.rs` / `preview_update/`：预览加载调度、结果应用、缓存保留与 UI defer。
- `preview_detail.rs`：session detail 预览异步渲染。
- `provider_test.rs`：Provider 连接测试异步请求与结果回收。
- `title_summary.rs`：Codex 标题总结异步请求、结果回收与落库。
- `async_ops_tests.rs`：异步状态保留与无变化扫描不刷新预览的回归测试。
