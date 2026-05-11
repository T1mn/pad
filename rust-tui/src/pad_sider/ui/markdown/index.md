# markdown

- `render.rs`：Markdown 渲染状态机，把 `pulldown-cmark` 事件转成 `ratatui::Text`。
- `events.rs`：处理 Markdown 事件、列表、引用、代码块与换行策略。
- `style.rs`：标题、行判断、行内代码与代码块样式辅助。

渲染策略保持紧凑，代码块隐藏 fence 语言标签，并按语言使用不同代码色。
