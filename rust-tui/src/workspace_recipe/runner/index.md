# workspace_recipe/runner

- `../runner.rs` 内联 `plan`：从 recipe 构建 tmux launch plan 和 browser URL 列表。
- `../runner.rs` 内联 `step`：把单个 step 转成 tmux new-session/new-window 命令。
- `../runner.rs` 内联 `execute`：执行 launch plan、打开 browser URL，并返回 run report。
