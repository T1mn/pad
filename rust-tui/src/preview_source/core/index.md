# core

- `model.rs`：PreviewRequest / PreviewUpdate 数据结构。
- `refresh.rs`：按 agent 状态、live/history/app origin 计算预览刷新间隔。
- `load.rs`：选择 tmux/session 预览源并组装 PreviewUpdate。
- `tmux.rs`：tmux pane fallback capture。
