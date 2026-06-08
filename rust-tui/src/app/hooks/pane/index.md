# app/hooks/pane

- `subagent.rs`：识别 Codex subagent hook，映射回父 session 并跳过后续状态更新。
- `panel_update.rs`：把 pane hook 应用到 panel 状态、session metadata、continuity 与 session cache。
- `effects.rs`：panel 更新后的 Claude history、完成通知与 Codex 标题摘要副作用。
