# tmux

- `../tmux.rs`：pad sider toggle 入口与 tmux 命令执行。
- `pane.rs` / `pane_tests.rs`：tmux pane 信息解析、存在性检查、focus 与 zoom 辅助。
- `helper.rs`：helper pane 创建、隐藏、显示和 focus/zoom。
- `options.rs` / `options_tests.rs`：target/helper pane 关联和 zoom restore 的 pane-local tmux option。
