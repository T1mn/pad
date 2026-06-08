# ui

- `mod.rs`：pad sider TUI 主循环，启用键盘与鼠标事件。
- `input.rs` / `input/`：快捷键、鼠标滚动与当前焦点动作分发。
- `render.rs` / `render/`：IDE 风格左右分栏渲染；左侧 header/tree/index map/Codex runs，右侧交给文件预览模块。
- `nav_window.rs`：左侧 tree/index/Codex runs 只构建可见行，避免大目录每帧全量 ListItem。
- `file_preview.rs` / `file_preview/` / `render_window.rs` / `render_window_tests.rs`：右侧文件/diff/代码预览渲染缓存与可见行窗口。
- `diff.rs` / `diff/`：Codex 单轮 patch 的结构化 diff 解析与 side-by-side / unified 配色渲染。
- `split.rs`：固定左侧可读宽度，把额外宽度优先留给 preview。
- `overlay.rs` / `overlay/`：全屏文件预览、搜索弹层与快捷键帮助，按弹层类型拆分。
- `line_numbers.rs` / `line_numbers_tests.rs`：按 `n` 给右侧预览与全屏预览补行号，默认隐藏。
- `text_zoom.rs`：预览内容紧凑 / 放大显示密度处理。
- `syntax.rs` / `syntax/`：VS Code Dark+ 风格多语言代码高亮。
- `file_icons.rs`：文件树语言短标签与强调色。
- `markdown.rs` / `markdown/` / `markdown_tests.rs`：紧凑 Markdown 渲染、样式模块与回归测试。
