# relay/opencode

- `managed.rs`：读写 pad 记录的 OpenCode provider keys，用于清理旧托管项。
- `provider.rs`：生成并同步 OpenCode `provider` 配置块。
- `model.rs`：同步 `model` / `small_model`，并移除旧托管引用。
