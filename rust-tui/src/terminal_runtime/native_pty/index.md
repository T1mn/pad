# terminal_runtime/native_pty

- `reader_io.rs`：PTY reader worker、bounded output/input queue、resize 与 nonblocking I/O。
- `process.rs`：owned child 的 signal escalation、deadline wait、reap 与 drop cleanup。
