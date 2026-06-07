# event

- `key_pipeline.rs`: key event routing by mode.
- `input_clear.rs`: Shift+Delete bulk-clear handling for active text inputs.
- `normal.rs`: normal mode key handling and preview Tab behavior.
- `mode_dispatch.rs`: thin forwarding layer for non-normal modes.
- `modes/`: per-mode key handlers, including notification inbox navigation.
- `mouse.rs` / `mouse_pipeline.rs`: mouse dispatch and scroll handling.
- `loop_core.rs` / `loop_state.rs`: main event loop state.
- `refresh_pipeline.rs`: periodic refresh and async result checks.
- `attach.rs`: tmux attach and focus helpers.
- `tests/`: event behavior regression tests.
