# sidebar/build/history_codex

- `../history_codex.rs` 内联 `merge`：读取当前 folder 的 Codex 历史线程，过滤 subagent 并合并到 folder。
- `../history_codex.rs` 内联 `entry`：把 `CodexThreadRef` 转成 `SidebarThread`。
- `../history_codex.rs` 内联 `snapshot`：把 session cache snapshot 应用到 Codex history thread。
