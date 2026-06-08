# preview_source/opencode

- `db.rs`：以只读 SQLite 方式读取 OpenCode message/part 表并组装 turns。
- `text.rs`：从 OpenCode JSON message/part 中识别 role 与文本内容。
- `tests.rs`：OpenCode SQLite transcript 解析回归测试。
