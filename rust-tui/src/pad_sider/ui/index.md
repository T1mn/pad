# ui

- `mod.rs`：pad sider TUI 主循环，启用键盘与鼠标事件。
- `input.rs`：快捷键、鼠标滚动与当前焦点动作分发。
- `render.rs`：左右分栏渲染；左侧 tree/index map/changes，右侧文件预览。
- `split.rs`：固定左侧可读宽度，把额外宽度优先留给 preview。
- `overlay.rs`：全屏 Markdown 预览、搜索弹层与快捷键帮助。
- `line_numbers.rs`：按 `n` 给右侧预览与全屏预览补行号，默认隐藏。
- `text_zoom.rs`：预览内容紧凑 / 放大显示密度处理。
- `markdown.rs` / `markdown/`：紧凑 Markdown 渲染，子目录内有独立索引。
