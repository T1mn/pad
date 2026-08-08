# telegram/hooks/pending_match

- `../pending_match.rs` 内联 `apply`：把 hook event 应用到 pending request，触发 awaiting_stop 或 completion。
- `../pending_match.rs` 内联 `matching`：按 pane、prompt hash、turn id 判断 pending request 是否匹配 hook event。
- `../pending_match.rs` 内联 `advance`：把 pending request 推进到 awaiting_stop，并记录 session/transcript/scan offset。
