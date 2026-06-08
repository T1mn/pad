# navigation

- `mod.rs`：导航模块入口与 sidebar 索引辅助函数。
- `cache.rs` / `cache/`：sidebar folder / visible item 缓存与启动排序种子。
- `selection.rs` / `selection/`：当前 sidebar item、preview thread、panel 与 tree 同步。
- `movement.rs` / `movement/`：上下移动、手动排序、数字跳转与 sidebar index 跳转。
- `folders.rs`：folder 展开/收起与批量切换。
- `space_action.rs`：空格延迟动作与双空格展开/收起。
- `tests/`：navigation 回归测试。
