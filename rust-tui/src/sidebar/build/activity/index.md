# sidebar/build/activity

- `merge.rs`：把历史 thread 合并进现有 live/history thread 列表。
- `../activity.rs` 内联 `overrides`：应用运行时 activity override 到 sidebar thread。
- `../activity.rs` 内联 `sort`：用运行时/启动时 activity 更新时间修正排序时间。
