# claude_history/scan

- `discover.rs`：递归查找 Claude JSONL 会话文件，并跳过 `subagents` 目录。
- `sync.rs`：按扫描结果增量同步 SQLite 索引、清理失效记录并记录扫描状态。
