# session

- `create.rs` / `create/`: high-level tmux agent session creation, status style and handoff logging.
- `target.rs`: tmux session/window creation, target parsing, selection and client switch.
- `../session.rs` 内联 `pad_context`: current PAD tmux pane/window/session lookup.
- `return_bindings.rs` / `return_bindings/`: F12/Ctrl+Q/F10/Ctrl+Tab return command, saved binding and sider toggle installation.
- `../session.rs` 内联 `bindings`: saved binding lookup, restore command and sider toggle command helpers.
- `../session.rs` 内联 `launch`: delayed agent launch for CLIs that need a live tmux client.
- `../session.rs` 内联 `shell`: shell quoting and trace logging command builders.
- `status.rs`: tmux status style override and restore calculation.
- `../session.rs` 内联 `tmux`: small tmux query helpers.
- `tests.rs`: focused regression tests for binding and status helpers.
