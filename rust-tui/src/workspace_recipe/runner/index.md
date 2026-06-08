# workspace_recipe/runner

- `plan.rs`：从 recipe 构建 tmux launch plan 和 browser URL 列表。
- `step.rs`：把单个 step 转成 tmux new-session/new-window 命令。
- `execute.rs`：执行 launch plan、打开 browser URL，并返回 run report。
