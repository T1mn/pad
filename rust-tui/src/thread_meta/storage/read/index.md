# storage/read

- `row.rs`：thread_meta 查询列定义与 SQLite row 到 ThreadMeta 的映射。
- `tags.rs`：thread_tags 批量装载、deleted 列表标签回填。
- `normalize.rs`：标题、note 与 tags 的读取后归一化。
