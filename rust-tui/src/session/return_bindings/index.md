# session/return_bindings

- `../return_bindings.rs` 内联 `context`：创建 session 后安装 return binding 所需的目标与 PAD 上下文。
- `../return_bindings.rs` 内联 `saved`：保存当前 F12/C-q 与 sider toggle root bindings，并生成恢复命令。
- `return_cmd.rs`：组装返回 PAD session/window/pane 的 tmux run-shell 命令。
- `../return_bindings.rs` 内联 `install`：安装 F12/C-q return bindings 和 sider toggle bindings。
