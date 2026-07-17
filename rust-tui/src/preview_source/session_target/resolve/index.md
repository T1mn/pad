# resolve

- `path.rs`：按 request/session id/provider 定位 transcript 或 OpenCode db 路径，Claude fallback 跟随 `CLAUDE_CONFIG_DIR`。
- `updated_at.rs`：从历史线程或 transcript mtime 计算目标更新时间。
- `panel.rs`：把 resolved `SessionTarget` 组装成可持久化的 `AgentPanel`。
