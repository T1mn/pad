# preview_source

- `mod.rs` / `core.rs` / `core/`：预览源入口、刷新节奏与 tmux/session 源选择；Session 成功路径返回结构化 turns，避免重复格式化全文。
- `session_loader.rs` / `session_loader/`：Session target 解析、transcript 解析、缓存回退与连续性保护。
- `claude.rs` / `claude/` / `codex.rs` / `codex/` / `gemini.rs` / `gemini/` / `grok.rs` / `grok/` / `opencode.rs` / `opencode/`：各 provider 会话解析；Codex 兼容冷 `.zst`，Grok 流式读取官方更新日志，OpenCode 读取官方 SQLite。
- `turns.rs` / `turns_tests.rs`：turn 级内容抽取。
- `session_target.rs` / `session_target/`：会话目标定位。
- `tests.rs` / `tests/`：预览源入口回归测试与 ignored 性能拆分基准。
