# runtime_status/tests

- `lock.rs`：跨线程、跨进程和崩溃释放的锁回归测试。
- `guard.rs`：状态 guard 清理、PID 复用和 zombie 判断回归测试。
- `../tests.rs` 内联 `support`：测试临时状态文件和锁文件清理辅助函数。
