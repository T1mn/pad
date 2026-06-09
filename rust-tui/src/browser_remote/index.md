# browser_remote

- `mod.rs`: facade，暴露 CLI、browser open 与 remote SSH 命令入口。
- `browser.rs` / `browser_tests.rs`: safe URL validation and OS browser open command.
- `remote.rs` / `remote_tests.rs`: SSH command builder for remote execution.
- `cli.rs` / `cli_tests.rs`: `pad __internal browser-remote ...` command entry.
