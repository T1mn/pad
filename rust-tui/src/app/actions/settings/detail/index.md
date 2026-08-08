# settings/detail

- `../detail.rs` 内联 `open`：settings 面板打开/关闭、搜索入口和列表状态重置。
- `../detail.rs` 内联 `kind`：当前 settings item 到 detail kind 的映射。
- `../detail.rs` 内联 `enter`：进入各 settings detail 前的状态初始化。
- `../detail.rs` 内联 `restore`：离开 detail、恢复预览临时状态和清理编辑态。
- `search_route.rs`：从 settings 搜索组合词初始化 detail 子层级，例如 `codex relay`、`codex cli`。
