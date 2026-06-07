# telegram

- `daemon.rs` / `api.rs`：bot 进程与 Telegram API 调用。
- `commands.rs` / `commands/`：命令入口与子命令实现。
- `callbacks.rs` / `render.rs`：回调处理与消息渲染。
- `hooks.rs` / `hooks/` / `pending.rs` / `state.rs`：hook、挂起任务与运行态。
- `help.rs` / `help/` / `locale.rs`：Telegram 帮助页、按钮、文案与多语言。
- `tests/`：Telegram 按状态、pending、journal、approval、help、history/restart 拆分的测试。
