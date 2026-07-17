# relay/tests/provider_configs

- `codex.rs`：Codex relay provider 完整性、配置写入、导入导出测试。
- `claude.rs` / `claude_safety.rs`：Claude settings env、配置目录与损坏配置保护测试。
- `deepseek.rs`：DeepSeek launcher 凭证内容与 Unix 权限测试。
- `gemini.rs`：Gemini `.env` / settings 写入和恢复测试。
- `opencode.rs` / `opencode_safety.rs`：OpenCode JSON/JSONC provider、model、managed state 与损坏配置保护测试。
