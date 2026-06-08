# socket_api

- `mod.rs`: socket API facade，只对外暴露 CLI 入口和 listener。
- `model.rs`: JSON request/response schema.
- `handler.rs` / `handler/` / `handler_tests.rs`: actions and tests for status, inbox, prompt, recipe, resume, browser and remote.
- `server.rs`: Unix socket JSONL listener at `~/.pad/pad-api.sock`.
- `client.rs`: JSONL client helper.
- `cli.rs`: `pad __internal socket-api ...` command entry.
