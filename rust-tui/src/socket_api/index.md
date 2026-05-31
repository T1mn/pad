# socket_api

- `mod.rs`: socket API public API.
- `model.rs`: JSON request/response schema.
- `handler.rs`: actions for status, inbox, prompt, recipe, resume, browser and remote.
- `server.rs`: Unix socket JSONL listener at `~/.pad/pad-api.sock`.
- `client.rs`: JSONL client helper.
- `cli.rs`: `pad __internal socket-api ...` command entry.
