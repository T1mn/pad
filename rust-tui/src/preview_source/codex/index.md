# codex

- `parser.rs`：Codex JSONL transcript 解析，用借用字段减少字符串复制，并只保留最近预览 turn。
- `tail.rs`：按尾部窗口读取 Codex JSONL，拿够最近预览需要的用户消息后停止，避免每次预览重扫超大 transcript。
