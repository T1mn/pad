# app

- `mod.rs`：`App` 状态、初始化、侧边栏缓存失效、session scope、thread list view 与内联时间 helper。
- `terminal.rs` / `terminal/`：UI 侧 Native terminal workspace；管理可持久化的 tab/二叉 split/profile/label、native agent sidebar registry、跨 tab focus，以及共享 controller 上每 pane 独立的 frame、输入、scroll、resize、关闭与公平背压重试。
- `socket_api.rs`：在主 UI 线程处理并测试 native pane 状态、输入、截屏与审批请求，外部服务不直接管理终端进程。
- `state/`：UI 与运行态结构。
- `actions.rs` / `actions/`：用户动作封装。
- `async_ops.rs`：预览、provider 测试、标题摘要与 CLI 检测异步入口。
- `async_ops/`：异步子功能。
- `preview.rs`：预览焦点、滚动、缓存控制。
- `navigation/`：面板、sidebar 列表、folder 与 tree 选择同步。
- `hooks.rs` / `hooks/`：hook 事件分发、pane/app-thread 状态应用、通知与历史同步。
- `clipboard.rs` / `clipboard/` / `clipboard_tests.rs`：系统剪贴板读写与 toast。
- `preview/`、`hooks/`、`async_ops/*_tests.rs`：对应模块测试。
