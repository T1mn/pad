# rollout

- `../rollout.rs` 内联 `collect`：递归扫描 `sessions` / `archived_sessions` 下需要同步 provider 的 rollout JSONL。
- `../rollout.rs` 内联 `rewrite`：只重写首行 `session_meta.payload.model_provider`。
- `apply.rs` / `apply_tests.rs`：校验首行未并发变化后原子写回 rollout 文件，并覆盖失败清理。
- `../rollout.rs` 内联 `line`：保留原换行符的首行切分工具。
- `../rollout.rs` 内联 `model`：待写回的 rollout 变更模型。
