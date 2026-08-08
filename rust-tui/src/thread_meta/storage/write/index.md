# storage/write

- `../write.rs` 内联 `meta`：写入手动标题、note 与 pinned 状态。
- `../write.rs` 内联 `generated`：写入自动生成标题和对应 turn count。
- `../write.rs` 内联 `deleted`：写入/清除删除状态和删除时间。
- `../write.rs` 内联 `tags`：替换 thread tags，负责去重、去空与事务提交。
- `../write.rs` 内联 `text`：写入前文本 trim 与空值归一化。
