# session/return_bindings

- `context.rs`：创建 session 后安装 return binding 所需的目标与 PAD 上下文。
- `saved.rs`：保存当前 F12/C-q 与 sider toggle root bindings，并生成恢复命令。
- `return_cmd.rs`：组装返回 PAD session/window/pane 的 tmux run-shell 命令。
- `install.rs`：安装 F12/C-q return bindings 和 sider toggle bindings。
