# async_ops/scan

- `change.rs`：判断扫描结果是否会影响侧边栏和预览刷新。
- `apply.rs`：合并扫描结果、保留 hook 状态和刷新 UI 缓存。
- `schedule.rs`：异步扫描启动、结果回收与延迟扫描调度。
