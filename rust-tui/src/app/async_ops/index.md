# app/async_ops

- `provider_test.rs`：Provider 连接测试异步请求与结果回收。
- `title_summary.rs`：Codex 标题总结异步请求、结果回收与落库。
- 扫描任务会顺手规范旧 PAD `CODEX_HOME` 写入的 Codex rollout 路径。
- `async_ops_tests.rs`：异步状态保留与无变化扫描不刷新预览的回归测试。
