# preview_source/codex/parser

- `model.rs`：借用反序列化用的 Codex transcript JSONL 结构。
- `lines.rs`：逐行解析 response item，并组装 preview turns。
- `message.rs`：用户/助手消息 content 文本抽取与图片/环境块归一化。
- `function_call.rs`：`spawn_agent` function call 事件摘要。
