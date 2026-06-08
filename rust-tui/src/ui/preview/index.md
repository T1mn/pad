# preview

- `layout.rs` / `layout/`：预览区域信息卡、鼠标选中文本抽取与命中检测。
- `welcome.rs`：无 pane 时的欢迎页和快捷键提示渲染。
- `session.rs` / `session/`：会话预览入口、列表/detail 渲染、滚动与 badge。
- `session_list_cache.rs` / `session_list_cache/`：会话列表按 turn allocation 缓存渲染结果，拆分构建、命中判断、行范围与可见行读取。
- `file_preview.rs` / `file_preview/`：文件预览、Markdown 分支与普通文本轻量语法高亮。
- `markdown.rs` / `markdown/`：Markdown 样式、detail 渲染、换行与 inline code 辅助函数。
- `plain.rs` / `plain/`：纯文本预览渲染、缓存、滚动和可见窗口。
- `common.rs`：预览公共配色与工具。
