# navigation/cache

- `folders.rs`：重建 sidebar folder 缓存，并把 preview cache 合并进 thread。
- `visible.rs`：重建可见 sidebar item 缓存和对外只读访问器。
- `startup.rs`：启动时从现有 thread 更新时间播种排序 activity。
