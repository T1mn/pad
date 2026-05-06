# markdown

- `render.rs`：Markdown 渲染状态机，把 `pulldown-cmark` 事件转成 `ratatui::Text`。
- `events.rs`：处理 Markdown 事件、列表、引用、代码块与换行策略。
- `style.rs`：标题、行判断等样式辅助。

渲染策略保持紧凑：普通块之间不额外加空行，只保留代码块内部真实空行。
