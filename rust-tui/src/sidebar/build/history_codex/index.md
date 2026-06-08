# sidebar/build/history_codex

- `merge.rs`：读取当前 folder 的 Codex 历史线程，过滤 subagent 并合并到 folder。
- `entry.rs`：把 `CodexThreadRef` 转成 `SidebarThread`。
- `snapshot.rs`：把 session cache snapshot 应用到 Codex history thread。
