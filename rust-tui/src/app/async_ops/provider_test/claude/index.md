# provider_test/claude

- `stream.rs`：读取 Claude Messages SSE，统计首个 text delta 与完整耗时。
- `model.rs` / `model_tests.rs`：选择真实对话 probe 使用的 Claude model 候选，并测试 Claude Code 常见别名展开。
- `../claude.rs` 内联 `response_text`：从非流式 Messages JSON 中抽取输出文本。
- `../claude.rs` 内联 `error`：HTTP/body 错误类别判断与结果文本截断。
