# claude

- `line.rs`：按 `SessionReadMode` 逐行读取 Claude JSONL transcript。
- `text.rs`：过滤 meta/tool/thinking 内容，并抽取 user / assistant 文本。
- `tests.rs`：Claude transcript 解析回归测试。
