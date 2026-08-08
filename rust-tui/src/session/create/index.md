# session/create

- `../create.rs`：创建 tmux agent session、安装返回 binding 并切换 client 的主流程。
- `../create.rs` 内联 `logging`：记录 tmux client handoff 上下文。
- `../create.rs` 内联 `status`：应用 agent session status bar 样式并保存恢复值。
