# attach

- `bindings.rs` / `bindings/`：临时 F12/C-q/F10/C-Tab root binding 安装、恢复与 return command 组装。
- `tmux.rs` / `tmux/`：tmux 命令执行、状态栏/窗口/pane 查询、shell quoting 与调试日志。
- `client_handoff.rs` / `client_handoff/`：同 tmux client 内跨/同 session 选择 window、pane 与 zoom handoff，子模块拆分步骤执行与日志。
- `pty_attach.rs`：无法 client handoff 时的 PTY attach fallback 与终端模式恢复。
