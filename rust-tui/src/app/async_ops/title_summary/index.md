# app/async_ops/title_summary

- `request.rs`：判断是否需要生成 Codex 标题，并发起异步 HTTP 摘要请求。
- `result.rs`：回收标题摘要结果、处理断连、落库并刷新 sidebar/preview。
- `support.rs`：标题摘要 channel 初始化与当前 Codex summary provider 选择。
