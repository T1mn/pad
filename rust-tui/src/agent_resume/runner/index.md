# agent_resume/runner

- `../runner.rs` 内联 `command`：按 agent 类型构建 resume shell command。
- `../runner.rs` 内联 `plan`：tmux session 名和启动命令计划。
- `execute.rs` / `tests.rs` 内联 `execute`：Codex runtime 准备与 tmux 命令执行。
- `display.rs` / `tests.rs` 内联 `display` / `../runner.rs` 内联 `shell`：dry-run 展示和 shell quoting/safe name helper。
- `tests.rs`：resume command 与 tmux launch plan 回归测试。
