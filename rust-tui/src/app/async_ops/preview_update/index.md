# preview_update

- `apply.rs` / `apply/`：把预览加载结果应用到 App 状态；子模块拆分状态、dirty 快照、thread cache 与 panel 回写。
- `cache.rs`：预览结果应用时的 detail/plain cache 保留判断。
- `defer.rs`：deferred UI 更新 flush 与当前选择匹配判断。
- `load.rs` / `load/`：预览加载任务调度、结果回收、导航 debounce 与刷新节流。
