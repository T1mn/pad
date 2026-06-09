# workspace_recipe

- `mod.rs`: facade，暴露 CLI、socket 所需的 load/find/run/display 入口。
- `model.rs` / `model_tests.rs`: TOML recipe schema and tmux-safe names.
- `storage.rs` / `storage_tests.rs`: `~/.pad/workspace-recipes.toml` parsing/loading.
- `runner.rs` / `runner/` / `runner_tests.rs`: dry-run plan、tmux step 命令构建、执行和入口测试。
- `cli.rs`: `pad __internal workspace-recipe ...` command entry.
- `display.rs` / `display_tests.rs`: recipe plan display helpers.
