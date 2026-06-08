# session_cache/persist

- `hook.rs`：从 hook event 合并最近问答、更新 binding，并记录 continuity 写入。
- `resolved.rs`：从解析出的 transcript turns 持久化 confirmed session cache。
- `record.rs`：按 agent session id 获取或创建缓存 record。
