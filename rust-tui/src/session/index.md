# session

- `create.rs` / `create/`: high-level tmux agent session creation, status style and handoff logging.
- `target.rs`: tmux session/window creation, target parsing, selection and client switch.
- `pad_context.rs`: current PAD tmux pane/window/session lookup.
- `return_bindings.rs` / `return_bindings/`: F12/Ctrl+Q/F10/Ctrl+Tab return command, saved binding and sider toggle installation.
- `bindings.rs`: saved binding lookup, restore command and sider toggle command helpers.
- `launch.rs`: delayed agent launch for CLIs that need a live tmux client.
- `shell.rs`: shell quoting and trace logging command builders.
- `status.rs`: tmux status style override and restore calculation.
- `tmux.rs`: small tmux query helpers.
- `tests.rs`: focused regression tests for binding and status helpers.
