# core

- `../core.rs` 内联 `model`：PreviewRequest / PreviewUpdate 数据结构。
- `../core.rs` 内联 `refresh`：按 agent 状态、live/history/app origin 计算预览刷新间隔。
- `load.rs`：选择 tmux/session 预览源并组装 PreviewUpdate。
- `../core.rs` 内联 `tmux`：tmux pane fallback capture。
