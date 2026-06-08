# gemini_history/tests

- `support.rs`：共享临时 root、DB 路径和 Gemini snapshot JSON 夹具。
- `archive.rs`：主/子 agent 快照优先级、归档和多项目同 session 归档测试。
- `query.rs`：按 cwd 查询和 project root 规范化测试。
- `scan.rs`：坏 snapshot 容错和源文件消失后的索引保留测试。
