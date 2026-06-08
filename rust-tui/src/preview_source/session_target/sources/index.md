# session_target/sources

- `codex.rs` / `claude.rs` / `gemini.rs` / `opencode.rs`：各 agent 的线程与 transcript 路径辅助查询。
- `resolved.rs`：从 request、历史线程和 live pane 推导 session id。
- `path.rs`：路径比较、mtime 与 JSONL 文件查找工具。
