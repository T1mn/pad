# socket_api/handler

- `../handler.rs` 内联 `core`：status、inbox、mark_read、prompt 基础 action；pane 操作由 UI 线程接管。
- `../handler.rs` 内联 `remote`：browser open 响应与 remote exec 参数校验；外部进程由 `../server.rs` 异步执行，避免阻塞 UI。
