# app

- `mod.rs`：`App` 状态与少量通用方法。
- `lifecycle.rs`：`App::new` 初始化。
- `display_scope.rs`：侧边栏缓存失效、live/all session scope 与 thread list view helper。
- `time.rs`：app 级时间戳与 handoff trace id helper。
- `state/`：UI 与运行态结构。
- `actions.rs` / `actions/`：用户动作封装。
- `async_ops.rs`：扫描、预览、provider 测试异步入口。
- `async_ops/`：异步子功能。
- `preview.rs`：预览焦点、滚动、缓存控制。
- `navigation/`：面板、sidebar 列表、folder 与 tree 选择同步。
- `hooks.rs` / `hooks/`：hook 事件分发、pane/app-thread 状态应用、通知与历史同步。
- `clipboard.rs` / `clipboard/` / `clipboard_tests.rs`：系统剪贴板读写与 toast。
- `preview/`、`hooks/`、`async_ops/*_tests.rs`：对应模块测试。
