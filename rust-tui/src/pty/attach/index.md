# pty/attach

- `../attach.rs`：Unix PTY attach 主流程与非 Unix stub。
- `input.rs`：stdin 转发到 PTY，并识别 F12/Ctrl+Q/Ctrl+C detach。
- `output.rs`：PTY 输出转发到 stdout。
