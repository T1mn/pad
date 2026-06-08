# workspace_recipe

- `mod.rs`: public API for recipe loading and running.
- `model.rs`: TOML recipe schema and tmux-safe names.
- `storage.rs`: `~/.pad/workspace-recipes.toml` parsing/loading.
- `runner.rs` / `runner/`: dry-run plan、tmux step 命令构建和执行。
- `cli.rs`: `pad __internal workspace-recipe ...` command entry.
