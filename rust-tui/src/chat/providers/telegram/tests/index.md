# telegram/tests

- `mod.rs` 内联 `core`：Telegram 基础文本、按钮与消息切分回归。
- `mod.rs` 内联 `approval`：Codex approval 扫描与回调数据回归。
- `mod.rs` 内联 `pending`：pending 请求状态、匹配与完成消息回归。
- `mod.rs` 内联 `journal`：pending journal 恢复时机回归。
- `history_restart.rs`：历史摘要与远程 restart 目标选择回归。
- `mod.rs` 内联 `help`：帮助页渲染与 callback payload 回归。
- `mod.rs` 内联 `state`：TelegramState 去重、ID、pending 映射回归。
- `mod.rs`：测试模块入口。
