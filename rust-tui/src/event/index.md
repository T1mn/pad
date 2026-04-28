# event

- `key_pipeline.rs`: key event routing by mode.
- `modes/`: per-mode key handlers.
- `mouse.rs` / `mouse_pipeline.rs`: mouse dispatch and scroll handling.
- `loop_core.rs` / `loop_state.rs`: main event loop state.
- `refresh_pipeline.rs`: periodic refresh and async result checks.
- `attach.rs`: tmux attach and focus helpers.
