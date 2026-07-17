# agent_resume

- `mod.rs`: facade，暴露 CLI 入口、socket lookup 和 resume launch。
- `model.rs`: resume target data model.
- `catalog.rs` / `catalog_tests.rs`: builds resume targets from `session_cache`，并直接发现 Grok 官方历史。
- `runner.rs` / `runner/`: agent-specific resume command, tmux launch plan, execution and shell quoting.
- `cli.rs`: `pad __internal agent-resume ...` command entry.
