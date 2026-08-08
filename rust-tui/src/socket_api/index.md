# socket_api

- `mod.rs`: socket API facade，只对外暴露 CLI 入口和 listener。
- `model.rs`: JSON request/response schema.
- `handler.rs` / `handler/` / `tests.rs` 内联 `handler`: actions and tests for status, inbox, prompt, recipe, resume, browser and remote.
- `server.rs`: Unix socket JSONL listener at `~/.pad/pad-api.sock`.
- `mod.rs` 内联 `client`: JSONL client helper.
- `mod.rs` 内联 `cli`: `pad __internal socket-api ...` command entry.
