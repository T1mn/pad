# terminal_runtime

- `model.rs`：pane ID、终端尺寸、cell、cursor、mode 与不可变 snapshot。
- `native_pty.rs`：无需 tmux 的本地 PTY 进程、输入输出、resize 与退出生命周期。
- `controller.rs`：独立后台 host、pane epoch、有界 UI command 与 latest immutable frame 发布。
- `engine.rs`：可替换的 `TerminalEngine`、factory 与多引擎 registry。
- `input.rs`：crossterm 按键与 bracketed paste 到 xterm 字节序列的无状态编码。
- `live_pane.rs`：组合 PaneRuntime 与 TransportRuntime 的 output/input/resize/exit 生命周期。
- `alacritty.rs`：基于 `alacritty_terminal` 的首个生产解析引擎。
- `transport.rs`：tmux/native PTY 共用的 transport command/event 协议。
- `transport_runtime.rs`：在独立线程托管 transport 的有界 command/event 通道与关闭协议。
- `widget.rs`：把 immutable terminal snapshot 和 pane label 绘制到 Ratatui buffer。
- `worker.rs`：按 pane 分片的有界多线程 engine runtime。
- `pane.rs`：label/engine/transport metadata 与 engine runtime 的组合层。
- `tests.rs`：ANSI、resize、多引擎和 worker 线程隔离测试。
- `stress_tests.rs`：8 panes 并发输出、resize、snapshot 与关闭压力回归。
