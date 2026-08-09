# runtime_status

- `../runtime_status.rs`：跨平台进程存活探测和 Unix zombie 状态判断。
- `identity.rs`：用 `started_at` 与 ps etime 对账，防止 PID 复用后误判/误杀。
- `identity_tests.rs`：etime 解析与身份对账回归测试。
- `../runtime_status.rs`：status lock 的跨平台文件锁和路径实现。
- `tests.rs`：status lock 并发所有权、guard 清理、PID 复用放行和 zombie stat 解析回归测试。
