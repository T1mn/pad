# pad_sider

- `mod.rs`：嵌入式入口分发，供 `pad __internal pad-sider ...` 调用。
- `cli.rs`：解析 `toggle` / `ui` 命令。
- `tmux.rs`：F10 / Ctrl+Tab 辅助栏的 split、隐藏、恢复与自动聚焦。
- `app.rs` / `actions.rs` / `actions/`：辅助栏状态、tree/changes 焦点、Markdown 预览、`index.md` 跳转与快捷键动作。
- `tree.rs`：tree 构建、递归文件扫描与忽略目录规则。
- `search.rs`：`/` 文件 fuzzy 搜索状态与匹配。
- `fs.rs`：git changed files、文件统计、相对路径与 Markdown 读取。
- `ui/mod.rs`：ratatui 主循环与按键分发。
- `ui/markdown.rs`：pad sider 的 Markdown 样式表与渲染入口。
- `ui/render.rs` / `ui/overlay.rs`：主布局、全屏预览与搜索弹层渲染。
