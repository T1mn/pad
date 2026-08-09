# browser_remote

- `mod.rs`: facade，暴露 CLI、browser open 与 remote SSH 命令入口。
- `mod.rs` / `tests.rs` 内联 `browser`: safe URL validation and OS browser open command.
- `mod.rs` / `tests.rs` 内联 `remote`: SSH command builder; host 走白名单校验 + `--` 终止选项解析。
- `mod.rs` / `tests.rs` 内联 `cli`: `pad __internal browser-remote ...` command entry.
