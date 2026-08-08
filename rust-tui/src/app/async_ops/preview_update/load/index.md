# preview_update/load

- `../load.rs` 内联 `trigger`：启动后台 preview 加载任务，处理已有任务时的 latest request queue。
- `../load.rs` 内联 `receive`：接收加载结果、丢弃 stale 结果、按 UI 状态应用或 defer。
- `../load.rs` 内联 `tick`：周期性检查是否需要触发 preview refresh，包含导航 debounce 与刷新节流。
