# pad_sider

- `mod.rs`：嵌入式入口分发，供 `pad __internal pad-sider ...` 调用。
- `cli.rs`：解析 `toggle` / `ui` 命令。
- `tmux.rs` / `tmux/` / `tmux_args.rs` / `tmux_codex_tests.rs`：F10 全屏 overlay 辅助栏、隐藏、恢复、tmux 参数日志格式与 zoom 状态处理。
- `codex_runs.rs` / `codex_runs_tests.rs`：Codex 单轮问答 diff 的右侧预览构建与测试。
- `app.rs` / `app/` / `actions.rs` / `actions/`：辅助栏状态、刷新/选择/预览状态、左侧导航模式、Codex runs diff 预览、全屏文件预览、`index.md` 跳转与快捷键动作。
- `ignore.rs`：tree、文件搜索与 index map 共用的忽略目录规则。
- `index_map.rs` / `index_map_tests.rs`：递归扫描项目内所有 `index.md`，用缩进和短目录名生成结构化索引地图及测试。
- `tree.rs` / `tree/`：tree 可见行模型、展开树构建、递归文件扫描与忽略目录规则。
- `search.rs` / `search/`：`/` 文件 fuzzy 搜索状态、键盘输入与匹配。
- `preview.rs`：右侧文件预览模型，区分 Markdown、文本、diff、目录与缺失文件。
- `preview_cache.rs`：右侧预览按路径、mtime 和大小缓存文件内容，记录慢加载日志。
- `preview_render_cache.rs` / `preview_render_cache_tests.rs`：缓存右侧预览已渲染行，滚动时只取可见窗口及缓存失效测试。
- `sizing.rs` / `sizing_tests.rs`：sider 45%-65% 小步宽度档位与 tmux resize。
- `fs.rs` / `fs_tests.rs`：文件统计、相对路径与完整文本读取。
- `ui/mod.rs`：ratatui 主循环，启用键盘与鼠标事件。
- `ui/markdown.rs`：pad sider 的 Markdown 样式表与渲染入口。
- `ui/render.rs` / `ui/overlay.rs` / `ui/diff.rs`：IDE 风格左右分栏、右侧文件/diff 预览、全屏预览与搜索弹层渲染。
