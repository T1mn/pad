# pad_sider

- `mod.rs`：嵌入式入口分发，供 `pad __internal pad-sider ...` 调用。
- `cli.rs`：解析 `toggle` / `ui` 命令。
- `tmux.rs`：F10 辅助栏的 split、隐藏、恢复与自动聚焦。
- `codex_runs.rs`：Codex 单轮问答 diff 的右侧预览构建。
- `app.rs` / `actions.rs` / `actions/`：辅助栏状态、左侧导航模式、Codex runs diff 预览、Markdown 预览、`index.md` 跳转与快捷键动作。
- `index_map.rs`：递归扫描项目内所有 `index.md`，用缩进和短目录名生成结构化索引地图。
- `tree.rs`：tree 构建、递归文件扫描与忽略目录规则。
- `search.rs`：`/` 文件 fuzzy 搜索状态与匹配。
- `preview.rs`：右侧文件预览模型，区分 Markdown、文本、diff、目录与缺失文件。
- `preview_cache.rs`：右侧预览按路径、mtime 和大小缓存文件内容，记录慢加载日志。
- `sizing.rs`：sider 45%-65% 小步宽度档位与 tmux resize。
- `fs.rs`：文件统计、相对路径与 Markdown / 文本读取。
- `ui/mod.rs`：ratatui 主循环，启用键盘与鼠标事件。
- `ui/markdown.rs`：pad sider 的 Markdown 样式表与渲染入口。
- `ui/render.rs` / `ui/overlay.rs` / `ui/diff.rs`：左右分栏、右侧文件/diff 预览、全屏预览与搜索弹层渲染。
