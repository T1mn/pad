# workspace_recipe

- `mod.rs`: facade，暴露 CLI、socket 所需的 load/find/run/display 入口。
- `model.rs`: TOML recipe schema and tmux-safe names.
- `storage.rs` / `storage_tests.rs`: `~/.pad/workspace-recipes.toml` parsing/loading.
- `runner.rs` / `runner/`: dry-run plan、tmux step 命令构建和执行。
- `cli.rs`: `pad __internal workspace-recipe ...` command entry.
