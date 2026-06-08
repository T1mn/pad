# telegram/daemon/process

- `embedded.rs`：pad 在线时的 embedded Telegram daemon 启动与守护。
- `external.rs`：standalone daemon 探活、启动、同步与重启入口。
- `stop.rs`：daemon 停止、进程退出等待、状态文件与 socket 清理。
