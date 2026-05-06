# ui

- `mod.rs`：pad sider TUI 主循环与按键分发。
- `render.rs`：左右分栏渲染；左侧 tree/index map/changes，右侧带行号文件内容。
- `overlay.rs`：全屏 Markdown 预览、搜索弹层与快捷键帮助。
- `line_numbers.rs`：给右侧预览与全屏预览补行号。
- `markdown.rs` / `markdown/`：紧凑 Markdown 渲染，子目录内有独立索引。
