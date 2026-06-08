# failures

- `detect.rs`：判断 Codex pending 是否该扫描失败、维护 transcript/offset 并生成失败 resolution。
- `reply.rs`：组装失败通知文本，包含 request、pane、目录与连续性信息。
