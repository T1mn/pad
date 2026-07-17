# Grok preview

- `../grok.rs`：流式读取 Grok Build 官方 `updates.jsonl`，合并文本 chunk。
- 损坏行、未知事件和未来字段会被跳过，不中断整段预览。
