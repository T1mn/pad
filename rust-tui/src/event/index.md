# event

- `event_pipeline.rs` / `key_pipeline.rs`: event/key routing by mode.
- `input_clear.rs` / `input_clear_tests.rs`: Shift+Delete bulk-clear handling for active text inputs.
- `normal.rs` / `normal/`: normal mode routing plus global, preview, sidebar, and Tab key helpers.
- `../event.rs` 内联 `mode_dispatch`: thin forwarding layer for non-normal modes.
- `modes.rs` / `modes/`: per-mode key handlers, including notification inbox navigation.
- `mouse.rs` / `mouse/` / `mouse_pipeline.rs` / `mouse_pipeline_tests.rs`: mouse dispatch facade、hit testing、click/selection/hover，以及 child mouse-reporting 与 PAD terminal scrollback 仲裁和回归测试。
- `../event.rs` 内联 `loop_core` / `../event.rs` 内联 `loop_state`: main event loop state.
- `refresh_pipeline.rs` / `refresh_pipeline/`：周期刷新、异步结果与 pipe/hook drain；每帧轮询 terminal controller，并用共享 split placement 批量 resize 当前 tab 的可见 panes。
- `attach.rs` / `attach/` / `attach_tests.rs`: tmux attach handoff、return bindings 与 focus helpers.
- `tests/`: event behavior regression tests.
