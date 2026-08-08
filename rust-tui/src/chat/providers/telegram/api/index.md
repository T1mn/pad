# api

- `../api.rs` 内联 `client`：Telegram HTTP client 单例、通用请求、错误包装与耗时日志。
- `../api.rs` 内联 `types`：Telegram API response/update/message DTO。
- `../api.rs` 内联 `text`：Telegram 长文本分片 helper。
- `../api.rs` 内联 `updates`：`getMe` / `getUpdates` 轮询入口。
- `../api.rs` 内联 `messages`：普通发送、长文本分片发送与编辑消息。
- `../api.rs` 内联 `interactive`：typing/draft/callback 等交互类 API。
- `../api.rs` 内联 `commands`：Bot 命令注册 payload 与 `setMyCommands`。
- `../api.rs` 内联 `chat_id`：chat id 字符串到 Telegram JSON 值的转换。
