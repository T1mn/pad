# state

- `model.rs`：TelegramState、selected target、agent snapshot、pending request 的 serde 模型。
- `pending.rs`：update 去重、pending request 查找与移除 helper。
- `../state.rs` 内联 `ids`：request/draft id 生成与时间戳 helper。
- `../state.rs` 内联 `storage`：Telegram state 与 hook journal 长度的磁盘 IO。
