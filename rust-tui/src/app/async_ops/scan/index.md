# async_ops/scan

- `../scan.rs` 内联 `change`：判断扫描结果是否会影响侧边栏和预览刷新。
- `../scan.rs` 内联 `apply`：合并扫描结果、保留 hook 状态和刷新 UI 缓存。
- `../scan.rs` 内联 `schedule`：异步扫描启动、结果回收与延迟扫描调度。
