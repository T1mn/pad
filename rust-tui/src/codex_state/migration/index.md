# codex_state/migration

- `paths.rs`：计算旧 PAD 私有 Codex home rollout 路径到官方 home 的前缀替换。
- `sqlite.rs`：打开迁移 SQLite、探测待迁移路径并执行前缀更新。
