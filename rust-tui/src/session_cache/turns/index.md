# session_cache/turns

- `../turns.rs` 内联 `merge`：把 hook prompt / assistant 文本合并进最近对话列表。
- `../turns.rs` 内联 `normalize`：清理、裁剪并可选归一化 cached preview turns。
- `../turns.rs` 内联 `prompt`：Codex cached prompt 的可选归一化与空值过滤。
