# preview_update/load

- `trigger.rs`：启动后台 preview 加载任务，处理已有任务时的 latest request queue。
- `receive.rs`：接收加载结果、丢弃 stale 结果、按 UI 状态应用或 defer。
- `tick.rs`：周期性检查是否需要触发 preview refresh，包含导航 debounce 与刷新节流。
