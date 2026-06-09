# settings/detail

- `open.rs`：settings 面板打开/关闭、搜索入口和列表状态重置。
- `kind.rs`：当前 settings item 到 detail kind 的映射。
- `enter.rs`：进入各 settings detail 前的状态初始化。
- `restore.rs`：离开 detail、恢复预览临时状态和清理编辑态。
- `search_route.rs`：从 settings 搜索组合词初始化 detail 子层级，例如 `codex relay`、`codex cli`。
