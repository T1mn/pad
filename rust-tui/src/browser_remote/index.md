# browser_remote

- `mod.rs`: facade，暴露 CLI、browser open 与 remote SSH 命令入口。
- `browser.rs` / `browser_tests.rs`: safe URL validation and OS browser open command.
- `remote.rs` / `remote_tests.rs`: SSH command builder; host 走白名单校验 + `--` 终止选项解析。
- `cli.rs` / `cli_tests.rs`: `pad __internal browser-remote ...` command entry.
