# opencode_history/util

- `db_paths.rs`：发现 OpenCode SQLite 路径，按 env、XDG/HOME 默认路径和 `opencode db path` 兜底去重。
- `sqlite.rs`：OpenCode SQLite 只读/写入连接和 rusqlite -> io error 转换。
