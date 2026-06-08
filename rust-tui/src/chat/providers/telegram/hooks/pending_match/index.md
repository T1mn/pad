# telegram/hooks/pending_match

- `apply.rs`：把 hook event 应用到 pending request，触发 awaiting_stop 或 completion。
- `matching.rs`：按 pane、prompt hash、turn id 判断 pending request 是否匹配 hook event。
- `advance.rs`：把 pending request 推进到 awaiting_stop，并记录 session/transcript/scan offset。
