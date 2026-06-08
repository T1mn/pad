# refresh_pipeline

- `events.rs`：drain hook / tmux pipe events，并根据 debounce 触发异步 scan。
- `draw.rs`：处理 terminal clear、draw 和慢帧日志。
