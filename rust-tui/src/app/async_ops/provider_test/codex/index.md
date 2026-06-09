# provider_test/codex

- `stream.rs`：读取 Codex Responses SSE，统计首个输出 delta 与完整耗时。
- `model.rs`：选择真实对话 probe 使用的 Codex model。
- `response_text.rs`：从非流式 Responses JSON 中抽取输出文本。
- `error.rs`：HTTP/body 错误类别判断与结果文本截断。
