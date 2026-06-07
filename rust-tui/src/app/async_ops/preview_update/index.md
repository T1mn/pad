# preview_update

- `apply.rs`：把预览加载结果应用到 App 状态。
- `cache.rs`：预览结果应用时的 detail/plain cache 保留判断。
- `defer.rs`：deferred UI 更新 flush 与当前选择匹配判断。
- `load.rs`：预览加载任务调度、结果回收与刷新节流。
