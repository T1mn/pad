# preview_source/codex/tests

- `../tests.rs` 内联 `support`：共享 Codex JSONL 临时文件路径 helper。
- `../tests.rs` 内联 `transcript`：Codex JSONL transcript 解析、上下文过滤、图片和 subagent 合并测试。
- `../tests.rs` 内联 `compressed`：冷 `.jsonl.zst` 与未来字段兼容测试。
- `../tests.rs` 内联 `normalize`：Codex 用户消息 normalize 规则测试。
- `../tests.rs` 内联 `status`：`/status` 输出里的 session id 抽取测试。
- `../tests.rs` 内联 `bench`：ignored 的 Codex transcript 解析本地基准。
