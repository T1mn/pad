# workspace_recipe

- `mod.rs`: facade，暴露 CLI、socket 所需的 load/find/run/display 入口。
- `model.rs` / `tests.rs` 内联 `model`: TOML recipe schema and tmux-safe names.
- `storage.rs` / `tests.rs` 内联 `storage`: `~/.pad/workspace-recipes.toml` parsing/loading.
- `runner.rs` / `runner/` / `tests.rs` 内联 `runner`: dry-run plan、tmux step 命令构建、执行和入口测试。
- `cli.rs`: `pad __internal workspace-recipe ...` command entry.
- `display.rs` / `tests.rs` 内联 `display`: recipe plan display helpers.
