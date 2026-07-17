# app/async_ops

- `scan.rs` / `scan/`：tmux pane 扫描调度、结果合并、变更判断和延迟扫描。
- `preview_update.rs` / `preview_update/`：预览加载调度、结果应用、缓存保留与 UI defer。
- `preview_detail.rs`：session detail 预览异步渲染。
- `provider_test.rs` / `provider_test/`：Provider 连接测试调度、HTTP probe 与结果回收。
- `title_summary.rs` / `title_summary/`：Codex 标题总结异步请求、provider 选择、结果回收与落库。
- `codex_cli.rs` / `codex_cli/`：Codex CLI 版本检测、原生更新后台任务与结果提示。
- `async_ops_tests.rs`：异步状态保留与无变化扫描不刷新预览的回归测试。
