# pipe

- `event.rs`：tmux control pipe 对外事件枚举。
- `listener.rs`：control mode 连接、重连、读取循环与事件发送。
- `client.rs` / `client_tests.rs`：tmux control-mode client 启动和旧 tmux flag fallback。
- `parser.rs`：tmux control protocol 行解析与忽略原因。
- `output.rs` / `output_tests.rs`：解析 `%output` / `%extended-output` 并还原 tmux 八进制字节流。
