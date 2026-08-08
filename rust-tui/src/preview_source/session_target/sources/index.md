# session_target/sources

- `codex.rs` / `codex_tests.rs` / `../sources.rs` 内联 `claude` / `gemini.rs` / `../sources.rs` 内联 `grok` / `../sources.rs` 内联 `opencode`：各 agent 的线程与 transcript 路径辅助查询；Codex 解析冷 `.zst` sibling。
- `resolved.rs`：从 request、历史线程和 live pane 推导 session id；live pane 仅在 cwd 唯一时回退。
- `path.rs`：路径比较、mtime 与 JSONL 文件查找工具。
- `path_tests.rs`：共享 cwd 唯一性回归测试。
