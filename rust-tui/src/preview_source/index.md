# preview_source

- `mod.rs`：预览源入口、刷新节奏与 tmux/session 源选择；Session 成功路径返回结构化 turns，避免重复格式化全文。
- `session_loader.rs`：Session target 解析、transcript 解析、缓存回退与连续性保护。
- `claude.rs` / `codex.rs` / `codex/` / `gemini.rs` / `opencode.rs`：各 provider 会话解析；Codex 预览用尾部读取和借用解析处理超大 JSONL，OpenCode 读取官方 SQLite。
- `turns.rs`：turn 级内容抽取。
- `session_target.rs`：会话目标定位。
- `tests.rs`：预览源入口与性能拆分基准测试。
