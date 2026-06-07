# preview

- `layout.rs` / `layout/`：预览区域信息卡、鼠标选中文本抽取与命中检测。
- `session.rs`：会话预览。
- `session_list_cache.rs`：会话列表按 turn allocation 缓存渲染结果，只取可见行绘制。
- `file_preview.rs`：文件预览与 Markdown 分支。
- `markdown.rs`：Markdown 样式与辅助函数。
- `plain.rs`：纯文本预览；绘制时只克隆可见窗口，避免长输出每帧全量复制。
- `common.rs`：预览公共配色与工具。
