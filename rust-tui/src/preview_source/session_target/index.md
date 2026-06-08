# session_target

- `resolve.rs` / `resolve/`：会话目标解析入口；子模块负责 transcript 路径、更新时间与 persistence panel 组装。
- `sources.rs` / `sources/`：Codex / Claude / Gemini / OpenCode 的线程、路径和 session id 辅助查询。
- `target.rs`：`SessionTarget` 结构体。
- `tests.rs`：Gemini 会话 ID、resolved session、persistence panel 回归测试。
