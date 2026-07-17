# claude_history/tests

- `support.rs`：共享临时目录、DB 路径和 Claude JSONL 写入夹具。
- `config_dir.rs`：自定义 `CLAUDE_CONFIG_DIR` 历史扫描测试。
- `parse.rs`：Claude transcript 解析、过滤 sidechain/progress/local-command 和 subagents 目录发现测试。
- `sync.rs`：增量同步、过期过滤和 thread lookup 测试。
- `archive.rs`：hook upsert 与归档过滤测试。
