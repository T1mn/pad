# api

- `client.rs`：Telegram HTTP client 单例、通用请求、错误包装与耗时日志。
- `types.rs`：Telegram API response/update/message DTO。
- `text.rs`：Telegram 长文本分片 helper。
- `updates.rs`：`getMe` / `getUpdates` 轮询入口。
- `messages.rs`：普通发送、长文本分片发送与编辑消息。
- `interactive.rs`：typing/draft/callback 等交互类 API。
- `commands.rs`：Bot 命令注册 payload 与 `setMyCommands`。
- `chat_id.rs`：chat id 字符串到 Telegram JSON 值的转换。
