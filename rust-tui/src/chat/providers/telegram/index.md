# telegram

- `daemon.rs` / `daemon/`：bot 主轮询循环、配置鉴权、pending 维护、update 分发、进程管理与状态落盘。
- `api.rs` / `api/`：Telegram API 调用、DTO 与长文本分片。
- `commands.rs` / `commands/`：命令入口与子命令实现。
- `callbacks.rs` / `callbacks/` / `render.rs`：回调处理、approval callback 与消息渲染。
- `hooks.rs` / `hooks/` / `pending.rs` / `state.rs` / `state/`：hook、挂起任务与运行态。
- `help.rs` / `help/` / `locale.rs` / `locale/`：Telegram 帮助页、按钮、文案与多语言。
- `tests/`：Telegram 按状态、pending、journal、approval、help、history/restart 拆分的测试。
