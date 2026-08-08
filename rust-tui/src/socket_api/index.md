# socket_api

- `mod.rs`: socket API facade，只对外暴露 CLI 入口和 listener。
- `model.rs`: JSON request/response schema.
- `handler.rs` / `handler/` / `tests.rs`：无需 UI 状态的 inbox、browser 与 remote action 校验和响应。
- `server.rs`：Unix socket JSONL listener at `~/.pad/pad-api.sock`；只将 pane 状态操作投递给 UI，外部进程走异步执行路径，并对捕获输出设置硬上限以保护 PAD 内存。
- `mod.rs` 内联 `client`: JSONL client helper.
- `mod.rs` 内联 `cli`: `pad __internal socket-api ...` command entry.
