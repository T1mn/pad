# client_handoff

- `steps.rs`：执行 switch-client、select-window、select-pane、resize-pane，并在失败时恢复 return bindings。
- `logging.rs`：输出 handoff 开始、步骤后和完成时的 tmux snapshot 日志。
