# telegram/daemon

- `run_loop.rs`：daemon 主循环，串联状态加载、配置鉴权、维护任务与 update 处理。
- `auth.rs`：Telegram 配置可用性检查、getMe 鉴权与命令注册。
- `maintenance.rs`：pending timeout/result、hook journal、rollout failure、approval 与 feedback 刷新。
- `updates.rs`：getUpdates 拉取、重复 update 过滤与 command/callback 分发。
- `process.rs` / `process/`：standalone/embedded daemon 启停、进程探活与 socket 清理。
- `state_io.rs`：TelegramState 序列化与变更落盘 helper。
