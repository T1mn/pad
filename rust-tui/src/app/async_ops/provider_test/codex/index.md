# provider_test/codex

- `stream.rs`：读取 Codex Responses SSE，统计首个输出 delta 与完整耗时。
- `../codex.rs` 内联 `model`：选择真实对话 probe 使用的 Codex model。
- `../codex.rs` 内联 `response_text`：从非流式 Responses JSON 中抽取输出文本。
- `../codex.rs` 内联 `error`：HTTP/body 错误类别判断与结果文本截断。
