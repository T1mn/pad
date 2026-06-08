# state

- `mod.rs`：主状态结构与模式枚举。
- `preview.rs` / `preview/`：预览态、滚动字段、缓存结构与 preview 小状态。
- `sidebar.rs` / `sidebar/`：侧边栏状态入口，拆分 thread action、space action、统计缓存与 `SidebarState`。
