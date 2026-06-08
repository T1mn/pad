# scan

- `parse.rs` / `parse/`：Gemini session JSON 解析入口，拆分 snapshot、project root 和消息文本归一化。
- `walk.rs`：递归查找 `session-*.json` 历史文件。
