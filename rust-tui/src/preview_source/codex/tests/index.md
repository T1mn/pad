# preview_source/codex/tests

- `support.rs`：共享 Codex JSONL 临时文件路径 helper。
- `transcript.rs`：Codex JSONL transcript 解析、上下文过滤、图片和 subagent 合并测试。
- `compressed.rs`：冷 `.jsonl.zst` 与未来字段兼容测试。
- `normalize.rs`：Codex 用户消息 normalize 规则测试。
- `status.rs`：`/status` 输出里的 session id 抽取测试。
- `bench.rs`：ignored 的 Codex transcript 解析本地基准。
