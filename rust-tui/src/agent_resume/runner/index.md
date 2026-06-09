# agent_resume/runner

- `command.rs`：按 agent 类型构建 resume shell command。
- `plan.rs`：tmux session 名和启动命令计划。
- `execute.rs` / `execute_tests.rs`：Codex runtime 准备与 tmux 命令执行。
- `display.rs` / `display_tests.rs` / `shell.rs`：dry-run 展示和 shell quoting/safe name helper。
- `tests.rs`：resume command 与 tmux launch plan 回归测试。
