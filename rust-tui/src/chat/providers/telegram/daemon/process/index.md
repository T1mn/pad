# telegram/daemon/process

- `../process.rs` 内联 `embedded`：pad 在线时的 embedded Telegram daemon 启动与守护。
- `external.rs`：standalone daemon 探活、启动、同步与重启入口。
- `stop.rs`：daemon 停止决策(身份对账后才发信号)、退出等待、匹配状态文件与 socket 清理。
- `stop_tests.rs`：停止决策、PID 复用和新状态保护的回归测试。
