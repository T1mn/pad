# telegram/locale

- `select.rs`：从全局配置解析 Telegram 使用的语言，并判断中文偏好。
- `text.rs` / `text/`：Telegram bot 命令、状态、错误、按钮等 key 文案表。
- `format.rs`：`tg_fmt*` 占位符替换 helpers。
- 上层 `locale.rs`：保持原 `tg` / `tg_fmt*` / locale helpers facade。
