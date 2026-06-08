# relay/permissions/codex

- `apply.rs`：应用 Codex runtime overlay：YOLO、Fast、features、web_search、status line、prompt file。
- `remove.rs`：按当前配置撤销 overlay，并恢复保存的原始 TOML 字段。
- `state.rs`：首次捕获和读取原始 Codex permission state。
- 上层 `codex.rs`：`CodexRuntimeOverlay` 参数结构与 apply/remove facade。
