# gemini_history/tests

- `../tests.rs` 内联 `support`：共享临时 root、DB 路径和 Gemini snapshot JSON 夹具。
- `../tests.rs` 内联 `archive`：主/子 agent 快照优先级、归档和多项目同 session 归档测试。
- `../tests.rs` 内联 `query`：按 cwd 查询和 project root 规范化测试。
- `../tests.rs` 内联 `scan`：坏 snapshot 容错和源文件消失后的索引保留测试。
