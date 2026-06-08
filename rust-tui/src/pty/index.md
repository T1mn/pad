# pty

- `keys.rs` / `keys_tests.rs`：识别 C-q/C-c/F12 的 raw、xterm、kitty 等按键编码。
- `capture.rs`：通过 `tmux capture-pane` 抓取 pane 文本并去 ANSI。
- `attach.rs` / `attach/`：Unix PTY attach 主流程、stdin/stdout 转发和非 Unix stub。
- 上层 `pty.rs`：保持原 public API 的轻量 facade。
