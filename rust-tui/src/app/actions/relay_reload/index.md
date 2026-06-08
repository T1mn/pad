# relay_reload

- `source.rs`：跟踪外部 relay config 的路径、mtime/len 变化，并处理编辑中延迟应用。
- `apply.rs`：比较外部配置差异、应用 agents/provider 字段并归一化 relay UI 选中状态。
