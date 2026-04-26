# app

- `mod.rs`：`App` 状态与初始化。
- `state/`：UI 与运行态结构。
- `actions.rs` / `actions/`：用户动作封装。
- `async_ops.rs`：扫描、预览、provider 测试异步入口。
- `async_ops/`：异步子功能。
- `preview.rs`：预览焦点、滚动、缓存控制。
- `navigation.rs`：面板与列表导航。
- `hooks.rs`：hook 事件应用与通知。
- `clipboard.rs`：复制与 toast。
- `preview/`、`hooks/`、`async_ops/*_tests.rs`：对应模块测试。
