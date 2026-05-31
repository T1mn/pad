# diff

- `model.rs`：结构化 diff 文档、文件、hunk 与行模型。
- `parse.rs`：把 git patch 解析成文件、hunk 与左右两侧行。
- `render.rs`：根据预览宽度选择 side-by-side 或增强 unified diff。
- `side_by_side.rs`：宽屏左右两栏 diff 渲染。
- `unified.rs`：窄屏单栏彩色 diff 渲染。
- `styles.rs`：diff 配色、行号格式与文本裁剪工具。
- `tests.rs`：解析、宽屏左右栏和窄屏单栏渲染测试。
