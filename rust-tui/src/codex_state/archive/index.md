# codex_state/archive

- `mutate.rs`：归档/恢复主流程，兼容 DB 保留 `.jsonl` 而磁盘已冷压缩为 `.jsonl.zst`，移动实际 rollout 后更新 DB。
- `db.rs`：读取线程状态与更新归档字段。
- `path.rs`：rollout 文件名、目录归属与恢复目标路径校验。
