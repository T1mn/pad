# storage/write

- `meta.rs`：写入手动标题、note 与 pinned 状态。
- `generated.rs`：写入自动生成标题和对应 turn count。
- `deleted.rs`：写入/清除删除状态和删除时间。
- `tags.rs`：替换 thread tags，负责去重、去空与事务提交。
- `text.rs`：写入前文本 trim 与空值归一化。
