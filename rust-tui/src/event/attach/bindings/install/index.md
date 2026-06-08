# attach/bindings/install

- `context.rs`：读取 handoff trace、当前 pad pane/session/window 上下文。
- `zoom.rs`：根据目标 pane 状态和配置判断是否需要临时 zoom，并生成恢复命令。
- `saved.rs`：保存 F12/C-q/F10/C-Tab 原 root binding 到 app。
- `return_cmd.rs`：组装 F12/C-q 返回 pad 时执行的恢复命令。
