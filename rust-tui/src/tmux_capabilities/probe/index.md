# probe

- `formats.rs` / `formats_tests.rs`：pane/display-message format 能力探测。
- `input.rs`：literal send-keys 与 bracketed paste 输入能力探测。
- `control.rs`：root key table、control-mode flag、focus-events 探测。
- `runtime.rs`：临时 tmux server 生命周期、tmux 命令执行 helper、探测 socket 时间戳。
