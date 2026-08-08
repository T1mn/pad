# refresh_pipeline

- `events.rs`：drain hook / tmux pipe events，并根据 debounce 触发异步 scan。
- `../refresh_pipeline.rs` 内联 `draw`：处理 terminal clear、draw 和慢帧日志。
