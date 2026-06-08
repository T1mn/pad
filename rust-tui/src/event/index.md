# event

- `event_pipeline.rs` / `key_pipeline.rs`: event/key routing by mode.
- `input_clear.rs` / `input_clear_tests.rs`: Shift+Delete bulk-clear handling for active text inputs.
- `normal.rs` / `normal/`: normal mode routing plus global, preview, sidebar, and Tab key helpers.
- `mode_dispatch.rs`: thin forwarding layer for non-normal modes.
- `modes.rs` / `modes/`: per-mode key handlers, including notification inbox navigation.
- `mouse.rs` / `mouse/` / `mouse_pipeline.rs`: mouse dispatch facade, hit testing, click/selection/hover/scroll handling.
- `loop_core.rs` / `loop_state.rs`: main event loop state.
- `refresh_pipeline.rs` / `refresh_pipeline/`: periodic refresh, async result checks, pipe/hook drain, and draw cycle.
- `attach.rs` / `attach/` / `attach_tests.rs`: tmux attach handoff、return bindings 与 focus helpers.
- `tests/`: event behavior regression tests.
