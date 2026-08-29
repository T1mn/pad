# ui

- `mod.rs`：主 UI 入口与总绘制调度。
- `layout.rs` / `layout_tests.rs` / `mod.rs`：整体布局计算。
- `panel_list.rs` / `panel_list/`：面板列表渲染。
- `codex_sidebar.rs`：Desktop Codex 风格 Sidebar 的 renderer-neutral 显示行适配，供 Tauri/WebKit 或原生 UI 消费。
- `preview.rs` / `preview/`：预览区渲染。
- `status_bar.rs` / `status_bar/` / `status_bar_tests.rs` / `mod.rs`：状态栏、测试与提示。
- `terminal.rs` / `terminal_tests.rs`：原生 terminal tab bar、递归 split 几何、pane 渲染、共享 hit-test placement 与回归测试。
- `modals/`：设置、relay、Telegram 等弹窗。
- `selection/`：选区模型与渲染。
