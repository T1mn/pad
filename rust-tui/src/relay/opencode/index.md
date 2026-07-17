# relay/opencode

- `managed.rs`：读写 pad 记录的 OpenCode provider keys，用于清理旧托管项。
- `provider.rs`：生成并同步 OpenCode `provider` 配置块。
- `model.rs`：同步 `model` / `small_model`，并移除旧托管引用。
- 写入前严格解析现有 JSON/JSONC；不支持的语法或损坏内容会原样保留。
