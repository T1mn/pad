# claude_history/tests

- `../tests.rs` 内联 `support`：共享临时目录、DB 路径和 Claude JSONL 写入夹具。
- `../tests.rs` 内联 `config_dir`：自定义 `CLAUDE_CONFIG_DIR` 历史扫描测试。
- `../tests.rs` 内联 `parse`：Claude transcript 解析、过滤 sidechain/progress/local-command 和 subagents 目录发现测试。
- `../tests.rs` 内联 `sync`：增量同步、过期过滤和 thread lookup 测试；用确定性文件时间覆盖排序，不做 wall-clock sleep。
- `../tests.rs` 内联 `archive`：hook upsert 与归档过滤测试。
