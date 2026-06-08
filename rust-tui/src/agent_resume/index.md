# agent_resume

- `mod.rs`: public API for resume target listing and launching.
- `model.rs`: resume target data model.
- `catalog.rs`: builds resume targets from `session_cache`.
- `runner.rs` / `runner/`: agent-specific resume command, tmux launch plan, execution and shell quoting.
- `cli.rs`: `pad __internal agent-resume ...` command entry.
