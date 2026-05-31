# preview_source

- `mod.rs`：预览源入口；Session 成功路径返回结构化 turns，避免重复格式化全文。
- `claude.rs` / `codex.rs` / `gemini.rs`：各 provider 会话解析。
- `turns.rs`：turn 级内容抽取。
- `session_target.rs`：会话目标定位。
